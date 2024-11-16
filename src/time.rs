use bevy::{
    ecs::schedule::ScheduleLabel,
    utils::{HashMap, HashSet},
};

pub use crate::prelude::*;

pub mod prelude {
    pub use super::{AppTimeExtension, EveryTime, RunEveryPlugin};
}

pub struct EveryTime {
    time_passed: Duration,
    pub every: Duration,
}

impl EveryTime {
    pub fn new(every: Duration, offset: Duration) -> Self {
        Self {
            time_passed: offset,
            every,
        }
    }

    pub fn tick(&mut self, time_passed: Duration) {
        self.time_passed += time_passed
    }

    // I'd like a &self version, so I could call tick(&mut self) then do a bunch of run(&self) and then finish_running(&mut self).
    pub fn run(&self, mut f: impl FnMut()) {
        let mut time_passed = self.time_passed;

        while time_passed > self.every {
            time_passed -= self.every;
            f();
        }
    }

    pub fn finish_running(&mut self) {
        while self.time_passed > self.every {
            self.time_passed -= self.every;
        }
    }

    pub fn full_run(&mut self, time_passed: Duration, mut f: impl FnMut()) {
        self.time_passed += time_passed;

        while self.time_passed > self.every {
            self.time_passed -= self.every;
            f();
        }
    }
}

pub struct RunEveryPlugin;

impl Plugin for RunEveryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AllRunEverys>()
            .add_systems(Update, run_run_every_schedule);
    }
}

#[derive(ScheduleLabel, Hash, Debug, Eq, PartialEq, Clone)]
pub struct RunEvery(pub Duration);

#[derive(Resource, Default)]
pub struct AllRunEverys(pub HashMap<RunEvery, Duration>);

pub trait AppTimeExtension {
    fn add_systems_that_run_every<M>(
        &mut self,
        every: Duration,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self;

    fn add_systems_that_run_every_with_offset<M>(
        &mut self,
        every: Duration,
        offset: Duration,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self;
}

impl AppTimeExtension for App {
    fn add_systems_that_run_every<M>(
        &mut self,
        every: Duration,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.add_systems(RunEvery(every), systems);
        self.world_mut()
            .resource_mut::<AllRunEverys>()
            .0
            .insert(RunEvery(every), default());
        self
    }

    fn add_systems_that_run_every_with_offset<M>(
        &mut self,
        every: Duration,
        offset: Duration,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.add_systems(RunEvery(every), systems);
        self.world_mut()
            .resource_mut::<AllRunEverys>()
            .0
            .insert(RunEvery(every), offset);
        self
    }
}

fn run_run_every_schedule(world: &mut World) {
    warn_once!("Time resource not fixed.");

    world.resource_scope::<AllRunEverys, ()>(|world, mut all_run_everys| {
        all_run_everys
            .0
            .iter_mut()
            .for_each(|(run_every, time_passed)| {
                //let time = world.remove_resource::<Time>().unwrap();
                *time_passed += world.resource::<Time>().delta();
                while *time_passed > run_every.0 {
                    *time_passed -= run_every.0;
                    world.run_schedule(run_every.clone());
                }
            });
    });
}

//MARK: Alternative
// somehow write a system config, likely as a closure, to have a local every to run the system. Idk.
macro_rules! run_every {
    ($every: expr, $systems: expr) => {
        || {}
    };
}
