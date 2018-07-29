use specs::{prelude::*, self, world::World};
use component::*;
use std::collections::{VecDeque, HashMap};
use std::ops;

// TODO: Can probably make a macro to spew out at least `Diff` and `Components`
// (TODO file structure...)
const DIFF_HISTORY_MAX_LEN: u32 = 60;

/// Holds history of diffs of the ECS, to be used by system::DiffSys
pub struct DiffHistory {
    frame: u32,
    diffs: VecDeque<Diff>,
    // state needed (holds the position read in the event streams)
    rem_id: ReaderId<RemovedFlag>,
    pos_mod_id: ReaderId<ModifiedFlag>,
    pos_ins_id: ReaderId<InsertedFlag>,
    shape_mod_id: ReaderId<ModifiedFlag>,
    shape_ins_id: ReaderId<InsertedFlag>,
    color_mod_id: ReaderId<ModifiedFlag>,
    color_ins_id: ReaderId<InsertedFlag>,
}
impl DiffHistory {
    pub fn new(world: &World) -> DiffHistory {
        let mut pos = world.write_storage::<Pos>();
        let mut shape = world.write_storage::<Shape>();
        let mut color = world.write_storage::<Color>();
        DiffHistory {
            frame: 0,
            diffs: VecDeque::default(),
            rem_id: pos.track_removed(),
            pos_mod_id: pos.track_modified(),
            pos_ins_id: pos.track_inserted(),
            shape_mod_id: shape.track_modified(),
            shape_ins_id: shape.track_inserted(),
            color_mod_id: color.track_modified(),
            color_ins_id: color.track_inserted(),
        }
    }
    /// Pushes a new diff to the history based on given components
    pub fn add_diff(&mut self, pos:   ReadStorage<Pos>,
                               shape: ReadStorage<Shape>,
                               color: ReadStorage<Color>) {
        self.frame += 1;
        let mut diff = Diff::default();
        pos.populate_modified(&mut self.pos_mod_id, &mut diff.pos);
        pos.populate_inserted(&mut self.pos_ins_id, &mut diff.pos);
        pos.populate_removed(&mut self.rem_id, &mut diff.removed);
        //^ NOTE: doesn't matter exactly what component we look at for removal
        
        shape.populate_modified(&mut self.shape_mod_id, &mut diff.shape);
        shape.populate_inserted(&mut self.shape_ins_id, &mut diff.shape);

        color.populate_modified(&mut self.color_mod_id, &mut diff.color);
        color.populate_inserted(&mut self.color_ins_id, &mut diff.color);
        self.diffs.push_back(diff);
    
        if self.diffs.len() > DIFF_HISTORY_MAX_LEN as usize {
            self.diffs.pop_front();
        }
    }

    fn get_diff_since(&self, since: u32, now: u32) -> Diff {
        let dt = now - since;
        assert!(dt <= DIFF_HISTORY_MAX_LEN);
        let mut diff = Diff::default();
        for d in self.diffs.iter().skip((DIFF_HISTORY_MAX_LEN - dt) as usize) {
            diff &= d;
        }
        diff
    }

    pub fn create_snapshot(&self, since: u32, now: u32, world: &World) -> Snapshot {
        let (id, shape, pos, color)
            = (world.read_storage::<UniqueId>(),
               world.read_storage::<Shape>(),
               world.read_storage::<Pos>(),
               world.read_storage::<Color>());
        let mut entities: HashMap<u32, Option<Entity>> = HashMap::new();

        // TODO handle since > DIFF_HISTORY_MAX_LEN
        let dt = now - since;
        if since == 0 || dt > DIFF_HISTORY_MAX_LEN {
            // FULL SNAPSHOT
            // TODO: Exactly the same as the `else`, but excluding the `diff` and `removed`
            for (id, pos) in (&id, &pos).join() {
                let id = id.0;
                if let Some(Some(e)) = entities.get_mut(&id) {
                    e.components.set_pos(pos);
                }
                if !entities.contains_key(&id) {
                    let mut components = Components::default();
                    components.set_pos(pos);
                    entities.insert(id, Some(Entity { components }));
                }
            }
            for (id, shape) in (&id, &shape).join() {
                let id = id.0;
                if let Some(Some(e)) = entities.get_mut(&id) {
                    e.components.set_shape(shape);
                }
                if !entities.contains_key(&id) {
                    let mut components = Components::default();
                    components.set_shape(shape);
                    entities.insert(id, Some(Entity { components }));
                }
            }
            for (id, color) in (&id, &color).join() {
                let id = id.0;
                if let Some(Some(e)) = entities.get_mut(&id) {
                    e.components.set_color(color);
                }
                if !entities.contains_key(&id) {
                    let mut components = Components::default();
                    components.set_color(color);
                    entities.insert(id, Some(Entity { components }));
                }
            }
        } else {
            // DIFF

            let diff = self.get_diff_since(since, now);
            // Handle removals
            for (id, _diff) in (&id, &diff.removed).join() {
                let id = id.0;
                entities.insert(id, None); // Signifies that entity `id` was removed
            }
            // Handle modifications & insertions
            for (id, pos, _diff) in (&id, &pos, &diff.pos).join() {
                let id = id.0;
                if let Some(Some(e)) = entities.get_mut(&id) {
                    e.components.set_pos(pos);
                }
                if !entities.contains_key(&id) {
                    let mut components = Components::default();
                    components.set_pos(pos);
                    entities.insert(id, Some(Entity { components }));
                }
            }
            for (id, shape, _diff) in (&id, &shape, &diff.shape).join() {
                let id = id.0;
                if let Some(Some(e)) = entities.get_mut(&id) {
                    e.components.set_shape(shape);
                }
                if !entities.contains_key(&id) {
                    let mut components = Components::default();
                    components.set_shape(shape);
                    entities.insert(id, Some(Entity { components }));
                }
            }
            for (id, color, _diff) in (&id, &color, &diff.color).join() {
                let id = id.0;
                if let Some(Some(e)) = entities.get_mut(&id) {
                    e.components.set_color(color);
                }
                if !entities.contains_key(&id) {
                    let mut components = Components::default();
                    components.set_color(color);
                    entities.insert(id, Some(Entity { components }));
                }
            }
        }
        Snapshot {
            reference: since,
            this: now,
            entities
        }
    }
}
/// What has been modified since last diff
#[derive(Default, Clone)]
pub struct Diff {
    removed: BitSet,
    pos: BitSet,
    shape: BitSet,
    color: BitSet,
}
impl<'a> ops::BitAndAssign<&'a Diff> for Diff {
    fn bitand_assign(&mut self, rhs: &'a Diff) {
        self.removed &= &rhs.removed;
        self.pos &= &rhs.pos;
        self.shape &= &rhs.shape;
        self.color &= &rhs.color;
    }
}




/// Incremental or entire snapshot of state at the server, to be sent to clients.
/// Also used by server to store what is known to be known by each client.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Snapshot {
    /// frame number of reference frame
    pub reference: u32,
    /// frame number of this frame
    pub this: u32,
    /// Hash map from (ID, type) to a list of components for this entity. If the value is None, it
    /// means deletion of this entity
    pub entities: HashMap<u32, Option<Entity>>,
}

/// Incremental or entire representation of an Entity
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entity{
    pub components: Components,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Components {
    pos: Option<Pos>,
    shape: Option<Shape>,
    color: Option<Color>,
}
impl Components {
    /// (Client) Insert new entity into ECS with components, with UniqueId `id`
    pub fn insert(self, updater: &specs::LazyUpdate, entities: &specs::world::EntitiesRes, id: u32) -> specs::Entity {
        info!("Insert entity");
        let mut builder = updater.create_entity(entities);
        match self.pos {
            Some(c) => builder = builder.with(c),
            None => warn!("Not all components present in received new entity")
        }
        match self.shape {
            Some(c) => builder = builder.with(c),
            None => warn!("Not all components present in received new entity")
        }
        match self.color {
            Some(c) => builder = builder.with(c),
            None => warn!("Not all components present in received new entity")
        }
        builder = builder.with(UniqueId (id));
        builder.build()
    }
    /// (Client) Apply to ECS system to some specific entity
    pub fn modify_existing(self, updater: &specs::LazyUpdate, entity: specs::Entity) {
        if let Some(c) = self.pos { updater.insert(entity, c); }
        if let Some(c) = self.shape { updater.insert(entity, c); }
        if let Some(c) = self.color { updater.insert(entity, c); }
    }

    pub fn set_pos(&mut self, pos: &Pos) {
        self.pos = Some(pos.clone());
    }
    pub fn set_shape(&mut self, shape: &Shape) {
        self.shape = Some(shape.clone());
    }
    pub fn set_color(&mut self, color: &Color) {
        self.color = Some(color.clone());
    }
}
