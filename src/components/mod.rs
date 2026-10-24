//! Componentes **parciais**: API com os nomes do Quasar, só as props e slots listados em
//! cada doc comment. As demais entram depois, sem quebrar o que existe.

mod badge;
mod banner;
mod btn;
mod card;
mod dialog;
mod empty_state;
mod icon;
mod layout;
mod list;
mod markup_table;
mod separator;
mod spinner;
mod tabs;
mod theme_switcher;

pub use badge::{Badge, BadgeAlign};
pub use banner::{Banner, BannerAction, BannerAvatar};
pub use btn::{Btn, BtnDesign, BtnType};
pub use card::{Card, CardActions, CardSection};
pub use dialog::{Dialog, DialogPosition};
pub use empty_state::{EmptyState, EmptyStateActions};
pub use icon::Icon;
pub use layout::{Drawer, DrawerSide, Header, Layout, Page, PageContainer, Toolbar, ToolbarTitle};
pub use list::{Item, ItemLabel, ItemSection, List};
pub use markup_table::{MarkupSeparator, MarkupTable};
pub use separator::Separator;
pub use spinner::Spinner;
pub use tabs::{Tab, TabPanel, TabPanels, Tabs};
pub use theme_switcher::ThemeSwitcher;
