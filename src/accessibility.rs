use crate::geometry::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityRole {
    Button,
    Checkbox,
    ComboBox,
    Dialog,
    Group,
    List,
    ListItem,
    Menu,
    MenuItem,
    Navigation,
    Image,
    ProgressIndicator,
    RadioGroup,
    RadioButton,
    Slider,
    Switch,
    Tab,
    TabList,
    TextField,
    SpinButton,
    StaticText,
    Status,
    Separator,
    Toolbar,
    Tooltip,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AccessibilityNode {
    pub role: AccessibilityRole,
    pub bounds: Rect,
    pub label: Option<String>,
    pub value: Option<String>,
    pub numeric_value: Option<f32>,
    pub numeric_minimum: Option<f32>,
    pub numeric_maximum: Option<f32>,
    pub enabled: bool,
    pub focusable: bool,
    pub focused: bool,
    pub selected: bool,
    pub checked: Option<bool>,
    pub invalid: bool,
    /// Restricts sequential keyboard focus to focusable nodes painted after
    /// this node until the scope disappears. Dialogs and popup surfaces use
    /// this to keep focus from escaping into obscured content.
    pub focus_scope: bool,
}

impl AccessibilityNode {
    pub const fn new(role: AccessibilityRole, bounds: Rect) -> Self {
        Self {
            role,
            bounds,
            label: None,
            value: None,
            numeric_value: None,
            numeric_minimum: None,
            numeric_maximum: None,
            enabled: true,
            focusable: false,
            focused: false,
            selected: false,
            checked: None,
            invalid: false,
            focus_scope: false,
        }
    }
}
