use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Score {
    name: String,
    objective: String,
}

impl Score {
    pub fn new<S1: Into<String>, S2: Into<String>>(name: S1, objective: S2) -> Self {
        Self {
            name: name.into(),
            objective: objective.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum DataSource {
    Block { block: String },
    Entity { entity: String },
    Storage { storage: String },
}

impl DataSource {
    pub fn block<S: Into<String>>(block: S) -> Self {
        Self::Block {
            block: block.into(),
        }
    }

    pub fn entity<S: Into<String>>(entity: S) -> Self {
        Self::Entity {
            entity: entity.into(),
        }
    }

    pub fn storage<S: Into<String>>(storage: S) -> Self {
        Self::Storage {
            storage: storage.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "action")]
pub enum ClickEvent {
    #[serde(rename = "open_url")]
    OpenUrl { value: String },
    #[serde(rename = "open_file")]
    OpenFile { value: String },
    #[serde(rename = "run_command")]
    RunCommand { value: String },
    #[serde(rename = "suggest_command")]
    SuggestCommand { value: String },
    #[serde(rename = "change_page")]
    ChangePage { value: String },
    #[serde(rename = "copy_to_clipboard")]
    CopyToClipboard { value: String },
}

impl ClickEvent {
    pub fn open_url<S: Into<String>>(value: S) -> Self {
        Self::OpenUrl {
            value: value.into(),
        }
    }

    pub fn open_file<S: Into<String>>(value: S) -> Self {
        Self::OpenFile {
            value: value.into(),
        }
    }

    pub fn run_command<S: Into<String>>(value: S) -> Self {
        Self::RunCommand {
            value: value.into(),
        }
    }

    pub fn suggest_command<S: Into<String>>(value: S) -> Self {
        Self::SuggestCommand {
            value: value.into(),
        }
    }

    pub fn change_page<S: Into<String>>(value: S) -> Self {
        Self::ChangePage {
            value: value.into(),
        }
    }

    pub fn copy_to_clipboard<S: Into<String>>(value: S) -> Self {
        Self::CopyToClipboard {
            value: value.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "action")]
pub enum HoverEvent {
    #[serde(rename = "open_url")]
    ShowText { contents: Box<Chat> },
}

impl HoverEvent {
    pub fn show_text(contents: Box<Chat>) -> Self {
        Self::ShowText { contents }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Style {
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    underlined: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    strikethrough: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    obfuscated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    insertion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    font: Option<String>,
    #[serde(rename = "hoverEvent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    hover_event: Option<HoverEvent>,
    #[serde(rename = "clickEvent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    click_event: Option<ClickEvent>,
}

impl Style {
    pub fn color<S: Into<String>>(&mut self, color: S) -> &mut Self {
        self.color = Some(color.into());
        self
    }

    pub fn bold(&mut self, bold: bool) -> &mut Self {
        self.bold = Some(bold);
        self
    }

    pub fn italic(&mut self, italic: bool) -> &mut Self {
        self.italic = Some(italic);
        self
    }

    pub fn underlined(&mut self, underlined: bool) -> &mut Self {
        self.underlined = Some(underlined);
        self
    }

    pub fn strikethrough(&mut self, strikethrough: bool) -> &mut Self {
        self.strikethrough = Some(strikethrough);
        self
    }

    pub fn obfuscated(&mut self, obfuscated: bool) -> &mut Self {
        self.obfuscated = Some(obfuscated);
        self
    }

    pub fn insertion<S: Into<String>>(&mut self, insertion: S) -> &mut Self {
        self.insertion = Some(insertion.into());
        self
    }

    pub fn font<S: Into<String>>(&mut self, font: S) -> &mut Self {
        self.font = Some(font.into());
        self
    }

    pub fn hover_event(&mut self, hover_event: HoverEvent) -> &mut Self {
        self.hover_event = Some(hover_event);
        self
    }

    pub fn click_event(&mut self, click_event: ClickEvent) -> &mut Self {
        self.click_event = Some(click_event);
        self
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct BaseChat {
    #[serde(skip_serializing_if = "Option::is_none")]
    extra: Option<Vec<Chat>>,
    #[serde(flatten)]
    style: Style,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Chat {
    Literal(String),
    ChatArr(Vec<Chat>),
    Text {
        text: String,
        #[serde(flatten)]
        base: BaseChat,
    },
    Translatable {
        translatable: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        with: Option<Vec<Box<Self>>>,
        #[serde(flatten)]
        base: BaseChat,
    },
    Score {
        score: Score,
        #[serde(flatten)]
        base: BaseChat,
    },
    Selector {
        selector: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<Box<Self>>,
        #[serde(flatten)]
        base: BaseChat,
    },
    Keybind {
        keybind: String,
        #[serde(flatten)]
        base: BaseChat,
    },
    NbtContents {
        nbt: String,
        interpret: bool,
        #[serde(flatten)]
        data_source: DataSource,
        #[serde(flatten)]
        base: BaseChat,
    },
}

impl mc_serializer::serde::Contextual for Chat {
    fn context() -> String {
        "Chat".to_string()
    }
}

impl Chat {
    pub fn extra(&mut self, extra: Vec<Chat>) {
        match self {
            Chat::Text { base, .. } => base.extra = Some(extra),
            Chat::Translatable { base, .. } => base.extra = Some(extra),
            Chat::Score { base, .. } => base.extra = Some(extra),
            Chat::Selector { base, .. } => base.extra = Some(extra),
            Chat::Keybind { base, .. } => base.extra = Some(extra),
            Chat::NbtContents { base, .. } => base.extra = Some(extra),
            _ => (),
        };
    }

    fn push_extra_single(base: &mut BaseChat, extra: Chat) {
        if let Some(present) = base.extra.as_mut() {
            present.push(extra);
        } else {
            base.extra = Some(vec![extra]);
        }
    }

    pub fn push_extra(&mut self, extra: Chat) {
        match self {
            Chat::Text { base, .. } => Self::push_extra_single(base, extra),
            Chat::Translatable { base, .. } => Self::push_extra_single(base, extra),
            Chat::Score { base, .. } => Self::push_extra_single(base, extra),
            Chat::Selector { base, .. } => Self::push_extra_single(base, extra),
            Chat::Keybind { base, .. } => Self::push_extra_single(base, extra),
            Chat::NbtContents { base, .. } => Self::push_extra_single(base, extra),
            _ => (),
        };
    }

    pub fn clear_extra(&mut self) {
        match self {
            Chat::Text { base, .. } => base.extra = None,
            Chat::Translatable { base, .. } => base.extra = None,
            Chat::Score { base, .. } => base.extra = None,
            Chat::Selector { base, .. } => base.extra = None,
            Chat::Keybind { base, .. } => base.extra = None,
            Chat::NbtContents { base, .. } => base.extra = None,
            _ => (),
        };
    }

    fn append_extra_single(base: &mut BaseChat, mut extra: Vec<Chat>) {
        if let Some(present) = base.extra.as_mut() {
            present.append(&mut extra);
        } else {
            base.extra = Some(extra);
        }
    }

    pub fn append_extra(&mut self, extra: Vec<Chat>) {
        match self {
            Chat::Text { base, .. } => Self::append_extra_single(base, extra),
            Chat::Translatable { base, .. } => Self::append_extra_single(base, extra),
            Chat::Score { base, .. } => Self::append_extra_single(base, extra),
            Chat::Selector { base, .. } => Self::append_extra_single(base, extra),
            Chat::Keybind { base, .. } => Self::append_extra_single(base, extra),
            Chat::NbtContents { base, .. } => Self::append_extra_single(base, extra),
            _ => (),
        };
    }

    pub fn modify_style<F: FnOnce(&mut Style) -> &mut Style>(&mut self, func: F) {
        match self {
            Chat::Text { base, .. } => (func)(&mut base.style),
            Chat::Translatable { base, .. } => (func)(&mut base.style),
            Chat::Score { base, .. } => (func)(&mut base.style),
            Chat::Selector { base, .. } => (func)(&mut base.style),
            Chat::Keybind { base, .. } => (func)(&mut base.style),
            Chat::NbtContents { base, .. } => (func)(&mut base.style),
            #[allow(clippy::needless_return)] // return is necessary to ignore type
            _ => return,
        };
    }

    pub fn literal<S: Into<String>>(string: S) -> Self {
        Self::Literal(string.into())
    }

    pub fn text<S: Into<String>>(string: S) -> Self {
        Self::Text {
            text: string.into(),
            base: BaseChat::default(),
        }
    }

    pub fn translatable<S: Into<String>>(translatable: S, with: Option<Vec<Box<Self>>>) -> Self {
        Self::Translatable {
            translatable: translatable.into(),
            with,
            base: BaseChat::default(),
        }
    }

    pub fn score(score: Score) -> Self {
        Self::Score {
            score,
            base: BaseChat::default(),
        }
    }

    pub fn selector<S: Into<String>>(selector: S, separator: Option<Box<Self>>) -> Self {
        Self::Selector {
            selector: selector.into(),
            separator,
            base: BaseChat::default(),
        }
    }

    pub fn keybind<S: Into<String>>(keybind: S) -> Self {
        Self::Keybind {
            keybind: keybind.into(),
            base: BaseChat::default(),
        }
    }

    pub fn nbt_contents<S: Into<String>>(
        path: S,
        interpret: bool,
        data_source: DataSource,
    ) -> Self {
        Self::NbtContents {
            nbt: path.into(),
            interpret,
            data_source,
            base: BaseChat::default(),
        }
    }
}
