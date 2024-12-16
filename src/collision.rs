use std::array;

use bevy::{ecs::query::QueryFilter, utils::Parallel};

pub use crate::prelude::*;

pub mod prelude {
    pub type ColliderGrid = super::ColliderGrid<GRID_WIDTH, GRID_HEIGHT>;

    pub use super::{
        check_collision, collide, distance_between_edges,
        CollisionQuestion as CollisionQuestionDeprecated,
        CollisionSensor as CollisionSensorDeprecated,
        DistanceSquaredBetweenEdgesQuestion as DistanceSquaredBetweenEdgesQuestionDeprecated,
        Radius, GRID_CELL_SIZE, GRID_HEIGHT, GRID_ORIGIN, GRID_WIDTH,
    };
}

pub const GRID_WIDTH: usize = 120;
pub const GRID_HEIGHT: usize = 25;
pub const GRID_CELL_SIZE: Vec2 = Vec2::new(500., 500.);
pub const GRID_ORIGIN: Vec2 = Vec2::new(-30_000., -2000.);

#[derive(Resource)]
pub struct ColliderGrid<const WIDTH: usize, const HEIGHT: usize>
where
    [(); WIDTH * HEIGHT]:,
{
    // Top left? I don't know!
    pub origin: Vec2,
    //#[cfg(debug_assertions)]
    //pub debug_directions: Box<[(Vec<u8>)]>,
    pub cells: Box<[(Vec<Entity>, Parallel<Vec<Entity>>); WIDTH * HEIGHT]>,
}

impl<const WIDTH: usize, const HEIGHT: usize> Default for ColliderGrid<WIDTH, HEIGHT>
where
    [(); WIDTH * HEIGHT]:,
{
    fn default() -> Self {
        Self {
            origin: GRID_ORIGIN,
            cells: Box::new(array::from_fn(|_| (vec![], default()))),
        }
    }
}

impl<const WIDTH: usize, const HEIGHT: usize> ColliderGrid<WIDTH, HEIGHT>
where
    [(); WIDTH * HEIGHT]:,
{
    pub fn new(origin: Vec2) -> Self {
        Self {
            origin,
            cells: Box::new(array::from_fn(|_| (vec![], default()))),
        }
    }

    pub fn get_collisions<T: QueryFilter>(
        &self,
        translation: Vec2,
        radius: f32,
        ignore: Option<Entity>,
        colliders: &Query<(&Radius, &Transform), T>,
    ) -> Collisions {
        let mut collisions = Collisions::default();

        let Some(grid_index) = self.translation_to_index(translation) else {
            return collisions;
        };

        self.cells[grid_index].0.iter().for_each(|other_entity| {
            if let Some(ignore) = ignore {
                if ignore == *other_entity {
                    return;
                }
            }

            let (other_collider, other_transform) = colliders.get(*other_entity).unwrap();

            if check_collision(
                radius,
                translation,
                other_collider.0,
                other_transform.translation.xy(),
            ) {
                collisions.add(*other_entity);
            }
        });
        collisions
    }

    pub fn collides_with_any<T: QueryFilter>(
        &self,
        translation: Vec2,
        radius: f32,
        ignore: Option<Entity>,
        colliders: &Query<(&Radius, &Transform), T>,
    ) -> bool {
        let Some(grid_index) = self.translation_to_index(translation) else {
            return false;
        };

        self.cells[grid_index].0.iter().any(|other_entity| {
            if let Some(ignore) = ignore {
                if ignore == *other_entity {
                    return false;
                }
            }

            let (other_collider, other_transform) = colliders.get(*other_entity).unwrap();

            if check_collision(
                radius,
                translation,
                other_collider.0,
                other_transform.translation.xy(),
            ) {
                return true;
            }

            false
        })
    }

    pub fn no_collisions_minimum_y_translation_with_limit<T: QueryFilter>(
        &self,
        translation: Vec2,
        radius: f32,
        limit: f32,
        ignore: Option<Entity>,
        colliders: &Query<(&Radius, &Transform), T>,
        //TODO: Replace f32 with Option<f32>
    ) -> f32 {
        // With enough translation, it might leave the current grid cell. Keep that in mind!

        // Very simplistic.
        let mut y_translation = 0.;
        loop {
            if y_translation > limit {
                info!("Went over limit. {}", y_translation);
                y_translation = f32::INFINITY;
                break;
            }

            // So unoptimised, perhaps consider y sorting, and throwing everything out that obviously isn't right?
            if self.collides_with_any(
                translation + Vec2::new(0., y_translation),
                radius,
                ignore,
                colliders,
            ) {
                y_translation += radius * 2.;
            } else {
                y_translation -= radius * 2.;
                let last_translation = translation + Vec2::new(0., y_translation);

                let Some(grid_index) = self.translation_to_index(last_translation) else {
                    todo!();
                };

                self.cells[grid_index].0.iter().for_each(|other_entity| {
                    if let Some(ignore) = ignore {
                        if ignore == *other_entity {
                            return;
                        }
                    }

                    let (other_collider, other_transform) = colliders.get(*other_entity).unwrap();

                    if other_transform.translation.y + other_collider.0
                        <= translation.y + y_translation - radius
                    {
                        return;
                    }

                    if check_collision(
                        radius,
                        last_translation,
                        other_collider.0,
                        other_transform.translation.xy(),
                    ) {
                        // This is true only for jumping to the center of the circle. Probably good enough though.
                        y_translation = other_transform.translation.y + other_collider.0
                            - last_translation.y
                            + radius;
                    }
                });
                break;
            }
        }

        return y_translation;
    }

    pub fn update(
        mut collider_grid: ResMut<ColliderGrid<WIDTH, HEIGHT>>,
        colliders: Query<(Entity, &Transform), With<Radius>>,
    ) {
        colliders.par_iter().for_each(|(entity, transform)| {
            // Do we want each entity to be in a neat box, or do we also want to push them into surrounding boxes, so that we only iterate one grid later?
            // For now: One grid later.

            let Some(index) = collider_grid.translation_to_index(transform.translation.xy()) else {
                warn_once!("Collider out of bounds. {:?}", transform.translation.xy());
                return;
            };

            // I don't know if any of this works. I'm just guessing. For now I'll go the slow route instead.

            // if ! index + WIDTH >= WIDTH * HEIGHT {
            //     collider_grid.cells[index + WIDTH].1.borrow_local_mut().push(entity);
            // }

            // if ! WIDTH > index {
            //     collider_grid.cells[index - WIDTH].1.borrow_local_mut().push(entity);
            // }

            // if ! index + 1 >= WIDTH * HEIGHT {
            //     collider_grid.cells[index + WIDTH].1.borrow_local_mut().push(entity);
            // }

            collider_grid.cells[index].1.borrow_local_mut().push(entity);

            // Top

            if let Some(index) = collider_grid.translation_to_index(
                transform.translation.xy() + Vec2::new(-GRID_CELL_SIZE.x, GRID_CELL_SIZE.y),
            ) {
                collider_grid.cells[index].1.borrow_local_mut().push(entity);
            }

            if let Some(index) = collider_grid
                .translation_to_index(transform.translation.xy() + Vec2::new(0., GRID_CELL_SIZE.y))
            {
                collider_grid.cells[index].1.borrow_local_mut().push(entity);
            }

            if let Some(index) = collider_grid.translation_to_index(
                transform.translation.xy() + Vec2::new(GRID_CELL_SIZE.x, GRID_CELL_SIZE.y),
            ) {
                collider_grid.cells[index].1.borrow_local_mut().push(entity);
            }

            // Middle

            if let Some(index) = collider_grid
                .translation_to_index(transform.translation.xy() + Vec2::new(GRID_CELL_SIZE.x, 0.))
            {
                collider_grid.cells[index].1.borrow_local_mut().push(entity);
            }

            if let Some(index) = collider_grid
                .translation_to_index(transform.translation.xy() + Vec2::new(-GRID_CELL_SIZE.x, 0.))
            {
                collider_grid.cells[index].1.borrow_local_mut().push(entity);
            }

            // Bottom

            if let Some(index) = collider_grid.translation_to_index(
                transform.translation.xy() + Vec2::new(-GRID_CELL_SIZE.x, -GRID_CELL_SIZE.y),
            ) {
                collider_grid.cells[index].1.borrow_local_mut().push(entity);
            }

            if let Some(index) = collider_grid
                .translation_to_index(transform.translation.xy() + Vec2::new(0., -GRID_CELL_SIZE.y))
            {
                collider_grid.cells[index].1.borrow_local_mut().push(entity);
            }

            if let Some(index) = collider_grid.translation_to_index(
                transform.translation.xy() + Vec2::new(GRID_CELL_SIZE.x, -GRID_CELL_SIZE.y),
            ) {
                collider_grid.cells[index].1.borrow_local_mut().push(entity);
            }
        });

        collider_grid.cells.par_iter_mut().for_each(|cell| {
            cell.0.clear();
            cell.1.iter_mut().for_each(|vec| cell.0.append(vec));
        });
    }

    pub fn translation_to_index(&self, translation: Vec2) -> Option<usize> {
        if translation.x < self.origin.x || translation.y < self.origin.y {
            return None;
        }

        let corrected_translation = (translation - self.origin) / GRID_CELL_SIZE;
        //info!("{}", corrected_translation);

        if corrected_translation.x >= WIDTH as f32 || corrected_translation.y >= HEIGHT as f32 {
            return None;
        }

        let float_index =
            corrected_translation.y.trunc() * WIDTH as f32 + corrected_translation.x.trunc();
        //info!(float_index);
        Some(float_index.trunc() as usize)
    }
}

// Trying to write a to a vec in parallel is not possible, no matter how hard we try.

#[derive(Default)]
pub struct Collisions(Vec<Entity>);

impl Collisions {
    pub fn iter(&self) -> impl Iterator<Item = &'_ Entity> {
        self.0.iter()
    }

    // Empties itself of all entities.
    fn clear(&mut self) {
        self.0.clear();
    }

    // Adds an entity.
    fn add(&mut self, entity: Entity) {
        self.0.push(entity);
    }
}

#[derive(Default)]
pub struct CollisionsWithDistanceSquaredBetweenEdges(Vec<(Entity, f32)>);

impl CollisionsWithDistanceSquaredBetweenEdges {
    pub fn iter(&self) -> impl Iterator<Item = &'_ (Entity, f32)> {
        self.0.iter()
    }

    // Empties itself of all entities.
    fn clear(&mut self) {
        self.0.clear();
    }

    // Adds an entity.
    fn add(&mut self, entity: Entity, distance: f32) {
        self.0.push((entity, distance));
    }
}

#[derive(Component)]
pub struct CollisionQuestion {
    pub translation: Vec2,
    pub radius: f32,

    answer: Option<Collisions>,
    answer_read: bool,
}

impl CollisionQuestion {
    pub fn new(translation: Vec2, radius: f32) -> Self {
        Self {
            translation,
            radius,
            answer: None,
            answer_read: false,
        }
    }

    pub fn answer(&mut self) -> Option<&Collisions> {
        let collisions = self.answer.as_ref()?;
        self.answer_read = true;
        Some(collisions)
    }
}

#[derive(Component)]
pub struct DistanceSquaredBetweenEdgesQuestion {
    pub translation: Vec2,
    pub radius: f32,

    answer: Option<CollisionsWithDistanceSquaredBetweenEdges>,
    answer_read: bool,
}

impl DistanceSquaredBetweenEdgesQuestion {
    pub fn new(translation: Vec2, radius: f32) -> Self {
        Self {
            translation,
            radius,
            answer: None,
            answer_read: false,
        }
    }

    pub fn answer(&mut self) -> Option<&CollisionsWithDistanceSquaredBetweenEdges> {
        let collisions = self.answer.as_ref()?;
        self.answer_read = true;
        Some(collisions)
    }
}

#[derive(Component, Default)]
pub struct CollisionSensor {
    pub collisions: Collisions,
}

impl CollisionSensor {
    pub fn iter(&self) {
        //self.collisions.
        // collisions.iter_mut().flat_map(|collisions| {
        //     collisions.iter()
        // });
    }
}

#[derive(Component)]
pub struct Radius(pub f32);

pub fn check_collision(
    radius: f32,
    translation: Vec2,
    other_radius: f32,
    other_translation: Vec2,
) -> bool {
    // let distance_squared = translation.distance_squared(other_translation);
    // let radii_sum = radius + other_radius;
    // let radii_difference = (radius - other_radius).abs();

    // radii_difference * radii_difference <= distance_squared
    //     && distance_squared <= radii_sum * radii_sum

    // I still am bad at math and physics... ChatGPT has helped me as always:

    let distance_squared = translation.distance_squared(other_translation);
    let radii_sum = radius + other_radius;

    distance_squared <= radii_sum * radii_sum
}

pub fn collide(
    collider_grid: Res<ColliderGrid<GRID_WIDTH, GRID_HEIGHT>>,
    colliders: Query<(Entity, &Radius, &Transform)>,
    mut sensors: Query<(Entity, &Radius, &Transform, &mut CollisionSensor)>,
    mut collision_questions: Query<(Entity, &mut CollisionQuestion)>,
    mut distance_questions: Query<(Entity, &mut DistanceSquaredBetweenEdgesQuestion)>,
    mut commands: Commands,
) {
    sensors
        .par_iter_mut()
        .for_each(|(entity, collider, transform, mut sensor)| {
            let Some(index) = collider_grid.translation_to_index(transform.translation.xy()) else {
                return;
            };

            warn_once!("We don't call clear on the collisions??");

            collider_grid.cells[index]
                .0
                .iter()
                .for_each(|other_entity| {
                    if entity == *other_entity {
                        return;
                    }

                    let (other_entity, other_collider, other_transform) =
                        colliders.get(*other_entity).unwrap();

                    if check_collision(
                        collider.0,
                        transform.translation.xy(),
                        other_collider.0,
                        other_transform.translation.xy(),
                    ) {
                        sensor.collisions.add(other_entity);
                    }
                });
        });

    collision_questions
        .iter_mut()
        .for_each(|(entity, mut question)| {
            if question.answer_read {
                commands.entity(entity).despawn();
                return;
            }

            let Some(index) = collider_grid.translation_to_index(question.translation) else {
                question.answer = Some(Collisions::default());
                return;
            };

            let mut collisions = Collisions::default();

            collider_grid.cells[index]
                .0
                .iter()
                .for_each(|other_entity| {
                    if entity == *other_entity {
                        return;
                    }

                    let (other_entity, other_collider, other_transform) =
                        colliders.get(*other_entity).unwrap();

                    if check_collision(
                        question.radius,
                        question.translation,
                        other_collider.0,
                        other_transform.translation.xy(),
                    ) {
                        collisions.add(other_entity);
                    }
                });

            question.answer = Some(collisions);
        });

    distance_questions
        .iter_mut()
        .for_each(|(entity, mut question)| {
            if question.answer_read {
                commands.entity(entity).despawn();
                return;
            }

            let Some(index) = collider_grid.translation_to_index(question.translation) else {
                question.answer = Some(default());
                return;
            };

            let mut collisions = CollisionsWithDistanceSquaredBetweenEdges::default();

            collider_grid.cells[index]
                .0
                .iter()
                .for_each(|other_entity| {
                    if entity == *other_entity {
                        return;
                    }

                    let (other_entity, other_collider, other_transform) =
                        colliders.get(*other_entity).unwrap();

                    if check_collision(
                        question.radius,
                        question.translation,
                        other_collider.0,
                        other_transform.translation.xy(),
                    ) {
                        let distance_squared = question
                            .translation
                            .distance_squared(other_transform.translation.xy());

                        let distance_squared_between_edges = distance_squared
                            - (question.radius * question.radius)
                            - (other_collider.0 * other_collider.0);
                        collisions.add(other_entity, distance_squared_between_edges);
                    }
                });

            question.answer = Some(collisions);
        });
}

pub fn distance_between_edges(
    radius: f32,
    translation: Vec2,
    other_radius: f32,
    other_translation: Vec2,
) -> f32 {
    let distance = translation.distance(other_translation);
    let sum_of_radii = radius + other_radius;

    if distance < sum_of_radii {
        0.
    } else {
        distance - sum_of_radii
    }
}

// Abandoned, as I couldn't get it to work...
// pub fn distance_squared_between_edges(
//     radius: f32,
//     translation: Vec2,
//     other_radius: f32,
//     other_translation: Vec2,
// ) -> f32 {
//     let distance_squared = translation.distance_squared(other_translation);
//     // Should this be squared(radius + other_radius) instead???
//     let sum_of_squared_radii = squared(radius + other_radius);
//     //let sum_of_squared_radii = squared(radius) + squared(other_radius);

//     if distance_squared < sum_of_squared_radii {
//         0.
//     } else {
//         let distance_squared_between_edges = distance_squared - sum_of_squared_radii;
//         distance_squared_between_edges
//     }

//     // Old code. The above code was what chatgpt thinks is improved. I did this cause I don't know how to search the internet yet.
//     // let distance_squared_between_edges =
//     //     distance_squared - (radius * radius) - (other_radius * other_radius);

//     // distance_squared_between_edges
// }

pub fn seperate_circle(
    radius_1: f32,
    translation_1: Vec2,
    radius_2: f32,
    translation_2: Vec2,
) -> Vec2 {
    let distance_x = translation_1.x - translation_2.x;
    let distance_y = translation_1.y - translation_2.y;
    // TODO: Squared distance?
    let distance = translation_1.distance(translation_2);
    let min_distance = radius_1 + radius_2;

    let unit_distance_x = distance_x / distance;
    let unit_distance_y = distance_y / distance;

    let overlap = min_distance - distance;
    let move_x = unit_distance_x * overlap;
    let move_y = unit_distance_y * overlap;

    Vec2::new(move_x, move_y)
}
