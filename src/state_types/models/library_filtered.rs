use super::{Ctx, LibraryLoadable};
use crate::state_types::*;
use crate::types::{LibItem, UID};
use derivative::*;
use itertools::Itertools;
use serde_derive::*;

#[derive(Derivative, Debug, Clone, PartialEq, Serialize)]
#[derivative(Default)]
#[serde(tag = "type", content = "content")]
pub enum LibraryState {
    #[derivative(Default)]
    NotLoaded,
    Loading(UID),
    Ready(UID),
}
#[derive(Default, Debug, Clone, PartialEq, Serialize)]
pub struct Selected {
    type_name: Option<String>,
}
pub type TypeNames = Vec<String>;
pub type LibItems = Vec<LibItem>;

#[derive(Default, Debug, Clone, Serialize)]
pub struct LibraryFiltered {
    pub library_state: LibraryState,
    pub selected: Selected,
    pub type_names: TypeNames,
    pub lib_items: LibItems,
}

impl<Env: Environment + 'static> UpdateWithCtx<Ctx<Env>> for LibraryFiltered {
    fn update(&mut self, ctx: &Ctx<Env>, msg: &Msg) -> Effects {
        match msg {
            Msg::Action(Action::Load(ActionLoad::LibraryFiltered { type_name })) => {
                let (selected, selected_effects) = reduce(
                    &self.selected,
                    SelectedAction::Select { type_name },
                    selected_reducer,
                    Effects::none(),
                );
                let (lib_items, lib_items_effects) = reduce(
                    &self.lib_items,
                    LibItemsAction::Select {
                        library: &ctx.library,
                        type_name,
                    },
                    lib_items_reducer,
                    Effects::none(),
                );
                self.selected = selected;
                self.lib_items = lib_items;
                selected_effects.join(lib_items_effects)
            }
            Msg::Event(Event::CtxChanged)
            | Msg::Event(Event::LibPersisted)
            | Msg::Internal(Internal::LibLoaded(_)) => {
                let (library_state, library_state_effects) = reduce(
                    &self.library_state,
                    LibraryStateAction::LibraryChanged {
                        library: &ctx.library,
                    },
                    library_state_reducer,
                    Effects::none(),
                );
                let (type_names, type_names_effects) = reduce(
                    &self.type_names,
                    TypeNamesAction::LibraryChanged {
                        library: &ctx.library,
                    },
                    type_names_reducer,
                    Effects::none(),
                );
                let (lib_items, lib_items_effects) = reduce(
                    &self.lib_items,
                    LibItemsAction::LibraryChanged {
                        library: &ctx.library,
                        type_name: &self.selected.type_name,
                    },
                    lib_items_reducer,
                    Effects::none(),
                );
                self.library_state = library_state;
                self.type_names = type_names;
                self.lib_items = lib_items;
                library_state_effects
                    .join(type_names_effects)
                    .join(lib_items_effects)
            }
            _ => Effects::none().unchanged(),
        }
    }
}

enum LibraryStateAction<'a> {
    LibraryChanged { library: &'a LibraryLoadable },
}
fn library_state_reducer(prev: &LibraryState, action: LibraryStateAction) -> (LibraryState, bool) {
    let next = match action {
        LibraryStateAction::LibraryChanged { library } => match library {
            LibraryLoadable::Ready(bucket) => LibraryState::Ready(bucket.uid.to_owned()),
            LibraryLoadable::Loading(uid) => LibraryState::Loading(uid.to_owned()),
            LibraryLoadable::NotLoaded => LibraryState::NotLoaded,
        },
    };
    let changed = prev.ne(&next);
    (next, changed)
}

enum SelectedAction<'a> {
    Select { type_name: &'a String },
}
fn selected_reducer(prev: &Selected, action: SelectedAction) -> (Selected, bool) {
    let next = match action {
        SelectedAction::Select { type_name } => Selected {
            type_name: Some(type_name.to_owned()),
        },
    };
    let changed = prev.ne(&next);
    (next, changed)
}

enum TypeNamesAction<'a> {
    LibraryChanged { library: &'a LibraryLoadable },
}
fn type_names_reducer(prev: &TypeNames, action: TypeNamesAction) -> (TypeNames, bool) {
    let next = match action {
        TypeNamesAction::LibraryChanged {
            library: LibraryLoadable::Ready(bucket),
        } => bucket
            .items
            .values()
            .filter(|x| !x.removed)
            .map(|x| x.type_name.to_owned())
            .unique()
            .collect(),
        _ => vec![],
    };
    let changed = prev.iter().ne(next.iter());
    (next, changed)
}

enum LibItemsAction<'a> {
    LibraryChanged {
        library: &'a LibraryLoadable,
        type_name: &'a Option<String>,
    },
    Select {
        library: &'a LibraryLoadable,
        type_name: &'a String,
    },
}
fn lib_items_reducer(prev: &LibItems, action: LibItemsAction) -> (LibItems, bool) {
    let next = match action {
        LibItemsAction::LibraryChanged {
            library: LibraryLoadable::Ready(bucket),
            type_name: Some(type_name),
        }
        | LibItemsAction::Select {
            library: LibraryLoadable::Ready(bucket),
            type_name,
        } => bucket
            .items
            .values()
            .filter(|item| !item.removed && item.type_name.eq(type_name))
            .cloned()
            .collect(),
        _ => vec![],
    };
    let changed = prev.iter().ne(next.iter());
    (next, changed)
}
