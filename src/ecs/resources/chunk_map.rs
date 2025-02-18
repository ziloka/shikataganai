use crate::ecs::components::blocks::Block;
use crate::ecs::components::chunk::{Chunk, ChunkTask};
use crate::ecs::resources::light::LightLevel;
use crate::util::array::{ImmediateNeighbours, DD, DDD};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::utils::{HashMap, HashSet};
use duplicate::duplicate_item;
use std::mem::MaybeUninit;

pub struct ChunkMeta {
  pub entity: Option<Entity>,
}

impl ChunkMeta {
  pub fn generating() -> Self {
    Self { entity: None }
  }
}

#[derive(SystemParam)]
pub struct BlockAccessorSpawner<'w, 's> {
  pub chunk_map: ResMut<'w, ChunkMap>,
  pub chunks: Query<'w, 's, &'static mut Chunk>,
  pub commands: Commands<'w, 's>,
}

#[derive(SystemParam)]
pub struct BlockAccessorStatic<'w, 's> {
  pub chunk_map: ResMut<'w, ChunkMap>,
  pub chunks: Query<'w, 's, &'static mut Chunk>,
}

#[derive(SystemParam)]
pub struct BlockAccessorReadOnly<'w, 's> {
  pub chunk_map: Res<'w, ChunkMap>,
  pub chunks: Query<'w, 's, &'static Chunk>,
}

pub trait BlockAccessorInternal<'w, 's> {
  fn get_chunk_entity_or_queue(&mut self, c: DDD) -> Option<Entity>;
}

impl<'w, 's> BlockAccessorInternal<'w, 's> for BlockAccessorStatic<'w, 's> {
  fn get_chunk_entity_or_queue(&mut self, c: DDD) -> Option<Entity> {
    let chunk_coord = ChunkMap::get_chunk_coord(c);
    match self.chunk_map.map.get(&chunk_coord) {
      None => None,
      Some(ChunkMeta { entity: None }) => None,
      Some(ChunkMeta { entity: Some(entity) }) => Some(*entity),
    }
  }
}

impl<'w, 's> BlockAccessorInternal<'w, 's> for BlockAccessorSpawner<'w, 's> {
  fn get_chunk_entity_or_queue(&mut self, c: DDD) -> Option<Entity> {
    let dispatcher = AsyncComputeTaskPool::get();

    let chunk_coord = ChunkMap::get_chunk_coord(c);
    match self.chunk_map.map.get(&chunk_coord) {
      None => {
        let task = dispatcher.spawn(Chunk::generate(chunk_coord));
        self.chunk_map.map.insert(chunk_coord, ChunkMeta::generating());
        self.commands.spawn().insert(ChunkTask {
          task,
          coord: chunk_coord,
        });
        None
      }
      Some(ChunkMeta { entity: None }) => None,
      Some(ChunkMeta { entity: Some(entity) }) => Some(*entity),
    }
  }
}

pub trait BlockAccessor {
  fn get_single(&mut self, c: DDD) -> Option<&Block>;
  fn get_mut(&mut self, c: DDD) -> Option<&mut Block>;
  fn get_many_mut<const N: usize>(&mut self, cs: [DDD; N]) -> Option<[&mut Block; N]>;
  fn get_light_level(&mut self, c: DDD) -> Option<LightLevel>;
  fn set_light_level(&mut self, c: DDD, light: LightLevel);
  fn propagate_light(&mut self, c: DDD, remesh: &mut HashSet<DD>);
}

impl<'w, 's> BlockAccessorReadOnly<'w, 's> {
  pub fn get_chunk_entity_or_queue(&self, c: DDD) -> Option<Entity> {
    let chunk_coord = ChunkMap::get_chunk_coord(c);
    match self.chunk_map.map.get(&chunk_coord) {
      None => None,
      Some(ChunkMeta { entity: None }) => None,
      Some(ChunkMeta { entity: Some(entity) }) => Some(*entity),
    }
  }

  pub fn get_single(&self, c: DDD) -> Option<&Block> {
    if c.1 < 0 || c.1 > 255 {
      return None;
    }
    self
      .get_chunk_entity_or_queue(c)
      .map(move |entity| &self.chunks.get(entity).unwrap().grid[c])
  }

  pub fn get_light_level(&self, c: DDD) -> Option<LightLevel> {
    if c.1 < 0 || c.1 > 255 {
      return None;
    }
    self
      .get_chunk_entity_or_queue(c)
      .map(|entity| {
        let chunk = self.chunks.get(entity).unwrap();
        (!chunk.grid[c].visible()).then_some(chunk.light_map[c])
      })
      .flatten()
  }
}

#[duplicate_item(T; [BlockAccessorSpawner]; [BlockAccessorStatic])]
// impl<'w, 's> BlockAccessor for BlockAccessorSpawner<'w, 's> {
impl<'w, 's> BlockAccessor for T<'w, 's> {
  fn get_single(&mut self, c: DDD) -> Option<&Block> {
    if c.1 < 0 || c.1 > 255 {
      return None;
    }
    self
      .get_chunk_entity_or_queue(c)
      .map(move |entity| self.chunks.get(entity).map(|ch| &ch.grid[c]).ok())
      .flatten()
  }
  fn get_mut(&mut self, c: DDD) -> Option<&mut Block> {
    if c.1 < 0 || c.1 > 255 {
      return None;
    }
    self
      .get_chunk_entity_or_queue(c)
      .map(move |entity| &mut self.chunks.get_mut(entity).unwrap().into_inner().grid[c])
  }
  fn get_many_mut<const N: usize>(&mut self, cs: [DDD; N]) -> Option<[&mut Block; N]> {
    for i in 0..N {
      for j in 0..i {
        if cs[i] == cs[j] {
          return None;
        }
      }
    }
    let mut chunk_entities: [Entity; N] = unsafe { MaybeUninit::uninit().assume_init() };
    for i in 0..N {
      let c = cs[i];
      if c.1 < 0 || c.1 > 255 {
        return None;
      }
      match self.get_chunk_entity_or_queue(c) {
        None => return None,
        Some(entity) => {
          chunk_entities[i] = entity;
        }
      }
    }
    Some(
      chunk_entities
        .map(|e| unsafe { self.chunks.get_unchecked(e).unwrap() })
        .into_iter()
        .enumerate()
        .map(|(i, c)| &mut c.into_inner().grid[cs[i]])
        .collect::<Vec<_>>()
        .try_into()
        .unwrap(),
    )
  }
  fn get_light_level(&mut self, c: DDD) -> Option<LightLevel> {
    if c.1 < 0 || c.1 > 255 {
      return None;
    }
    self
      .get_chunk_entity_or_queue(c)
      .map(|entity| {
        let chunk = self.chunks.get(entity).unwrap();
        (!chunk.grid[c].visible()).then_some(chunk.light_map[c])
      })
      .flatten()
  }
  fn set_light_level(&mut self, c: DDD, light: LightLevel) {
    self.get_chunk_entity_or_queue(c).map(|entity| {
      let mut chunk = self.chunks.get_mut(entity).unwrap();
      if !chunk.grid[c].visible() {
        chunk.light_map[c] = light;
      }
    });
  }
  fn propagate_light(&mut self, c: DDD, remesh: &mut HashSet<DD>) {
    let mut queue = vec![c];
    while !queue.is_empty() {
      let c = queue.pop().unwrap();
      if self.get_single(c).map_or(false, |e| e.visible()) {
        self.set_light_level(c, LightLevel::dark());
        remesh.insert(ChunkMap::get_chunk_coord(c));
        continue;
      }
      if let Some(current_light) = self.get_light_level(c) {
        let mut new_heaven_light = None;
        let mut new_hearth_light = None;
        for heaven_check in c.immediate_neighbours() {
          if let Some(LightLevel { mut heaven, hearth }) = self.get_light_level(heaven_check) {
            if heaven_check.1 - c.1 == 1 && heaven == 15 {
              heaven += 1
            }
            // TODO: fix this clusterfuck
            let new = if heaven - 1 > 16 { 0 } else { heaven - 1 };
            if current_light.heaven < heaven - 1 && heaven > 0 && new_heaven_light.map_or(true, |x| new > x) {
              new_heaven_light = Some(new);
            }
            let new = if hearth - 1 > 16 { 0 } else { hearth - 1 };
            if current_light.hearth < hearth - 1 && hearth > 0 && new_hearth_light.map_or(true, |x| new > x) {
              new_hearth_light = Some(new);
            }
          }
        }
        if new_heaven_light.is_none() && new_hearth_light.is_none() {
          continue;
        }
        let new_light = LightLevel::new(
          new_heaven_light.unwrap_or(current_light.heaven),
          new_hearth_light.unwrap_or(current_light.hearth),
        );
        self.set_light_level(c, new_light);
        let chunk_coord = ChunkMap::get_chunk_coord(c);
        remesh.insert(chunk_coord);
        for i in c.immediate_neighbours() {
          if let Some(LightLevel { heaven, hearth }) = self.get_light_level(i) {
            if (heaven >= new_light.heaven - 1 || new_light.heaven == 0)
              && (hearth >= new_light.hearth - 1 || new_light.hearth == 0)
            {
              continue;
            }
          }
          remesh.insert(ChunkMap::get_chunk_coord(i));
          queue.push(i);
        }
      }
    }
  }
}

pub struct ChunkMap {
  pub map: HashMap<DD, ChunkMeta>,
}

impl FromWorld for ChunkMap {
  fn from_world(_world: &mut World) -> Self {
    Self { map: HashMap::new() }
  }
}

impl ChunkMap {
  pub fn get_chunk_coord(mut coord: DDD) -> DD {
    if coord.0 < 0 {
      coord.0 -= 15;
    }
    if coord.2 < 0 {
      coord.2 -= 15;
    }
    (coord.0 / 16, coord.2 / 16)
  }
}
