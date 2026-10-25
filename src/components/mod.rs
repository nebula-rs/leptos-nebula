//! Componentes **parciais**: API com os nomes do Quasar, só as props e slots listados em
//! cada doc comment. As demais entram depois, sem quebrar o que existe.

mod badge;
mod banner;
mod btn;
mod btn_toggle;
mod card;
mod dialog;
mod empty_state;
mod expansion_item;
mod icon;
mod layout;
mod linear_progress;
mod list;
mod markup_table;
mod menu;
pub(crate) mod popup;
mod separator;
mod spinner;
mod tabs;
mod theme_switcher;
mod tooltip;

pub use badge::{Badge, BadgeAlign};
pub use banner::{Banner, BannerAction, BannerAvatar};
pub use btn::{Btn, BtnDesign, BtnType};
pub use btn_toggle::{BtnToggle, BtnToggleOption};
pub use card::{Card, CardActions, CardSection};
pub use dialog::{Dialog, DialogPosition};
pub use empty_state::{EmptyState, EmptyStateActions};
pub use expansion_item::{ExpansionItem, ExpansionItemHeader};
pub use icon::Icon;
pub use layout::{
    BOTTOM_BAR_SPACE, BottomBar, Drawer, DrawerSide, Footer, Header, Layout, Page, PageContainer,
    Toolbar, ToolbarTitle,
};
pub use linear_progress::{LinearProgress, ProgressSegment};
pub use list::{Item, ItemLabel, ItemSection, List};
pub use markup_table::{MarkupSeparator, MarkupTable};
pub use menu::Menu;
pub use separator::Separator;
pub use spinner::Spinner;
pub use tabs::{Tab, TabPanel, TabPanels, Tabs};
pub use theme_switcher::ThemeSwitcher;
pub use tooltip::Tooltip;
