// pub use crate::prelude::*;

// pub mod prelude {
//     pub use super::ComponentEvent;
// }

// #[derive(Default)]
// pub struct ComponentEvent<T, const HISTORY_LENGTH: usize = 3> {
//     next_index: usize,
//     events: ArrayVec<T, HISTORY_LENGTH>,
// }

// impl<T, const HISTORY_LENGTH: usize> ComponentEvent<T, HISTORY_LENGTH> {
//     pub fn write(&mut self, event: T) {
//         self.events[self.next_index % HISTORY_LENGTH] = event;
//         self.next_index += 1;
//     }

//     pub fn read(&self, next_index: &mut usize) -> ComponentEventIterator<T, HISTORY_LENGTH> {
//         let next_remembered_index = next_index
//             .clone()
//             .max(self.current_index.saturating_sub(HISTORY_LENGTH - 1));
//         *next_index = self.current_index + 1;

//         ComponentEventIterator {
//             component_event: self,
//             next_remembered_index,
//         }
//     }
// }

// pub struct ComponentEventIterator<'a, T, const HISTORY_LENGTH: usize> {
//     component_event: &'a ComponentEvent<T, HISTORY_LENGTH>,
//     next_remembered_index: usize,
// }

// impl<'a, T, const HISTORY_LENGTH: usize> Iterator
//     for ComponentEventIterator<'a, T, HISTORY_LENGTH>
// {
//     type Item = &'a T;

//     fn next(&mut self) -> Option<Self::Item> {
//         if self.next_remembered_index > self.component_event.current_index {
//             None
//         } else {
//             Some(&self.component_event.events[self.next_remembered_index % HISTORY_LENGTH])
//         }
//     }

//     fn size_hint(&self) -> (usize, Option<usize>) {
//         let exact_size = self.component_event.current_index - self.next_remembered_index + 1;
//         (exact_size, Some(exact_size))
//     }
// }

// impl<'a, T, const HISTORY_LENGTH: usize> ExactSizeIterator
//     for ComponentEventIterator<'a, T, HISTORY_LENGTH>
// {
//     fn len(&self) -> usize {
//         self.component_event.current_index - self.next_remembered_index + 1
//     }
// }
