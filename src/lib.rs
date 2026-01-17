#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use std::collections::HashMap;
use std::fmt;
use std::io::{self, Write};
use std::path::Path;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur when parsing or validating desktop entry files.
#[derive(Debug, Clone, PartialEq)]
pub enum DesktopEntryError {
    /// IO error during file reading/writing
    Io(String),
    /// File is not valid UTF-8
    InvalidUtf8,
    /// Missing required [Desktop Entry] group
    MissingDesktopEntryGroup,
    /// Duplicate group header
    DuplicateGroup(String),
    /// Invalid line format (not a comment, blank, group header, or key=value)
    InvalidLine(usize, String),
    /// Invalid group header format
    InvalidGroupHeader(usize, String),
    /// Invalid key name (must be ASCII A-Za-z0-9-)
    InvalidKeyName(usize, String),
    /// Missing required key
    MissingRequiredKey(String),
    /// Invalid value type
    InvalidValue(String, String),
    /// Validation error
    ValidationError(String),
}

impl fmt::Display for DesktopEntryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "IO error: {}", msg),
            Self::InvalidUtf8 => write!(f, "File is not valid UTF-8"),
            Self::MissingDesktopEntryGroup => {
                write!(f, "Missing required [Desktop Entry] group")
            }
            Self::DuplicateGroup(name) => write!(f, "Duplicate group: [{}]", name),
            Self::InvalidLine(line, content) => {
                write!(f, "Invalid line {} format: {}", line, content)
            }
            Self::InvalidGroupHeader(line, content) => {
                write!(f, "Invalid group header at line {}: {}", line, content)
            }
            Self::InvalidKeyName(line, name) => {
                write!(f, "Invalid key name at line {}: '{}'", line, name)
            }
            Self::MissingRequiredKey(key) => write!(f, "Missing required key: {}", key),
            Self::InvalidValue(key, reason) => {
                write!(f, "Invalid value for key '{}': {}", key, reason)
            }
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for DesktopEntryError {}

impl From<io::Error> for DesktopEntryError {
    fn from(err: io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

/// Result type for desktop entry operations.
pub type Result<T> = std::result::Result<T, DesktopEntryError>;

// ============================================================================
// Locale
// ============================================================================

/// Represents a locale identifier in the format `lang_COUNTRY.ENCODING@MODIFIER`.
///
/// According to the spec, the `_COUNTRY`, `.ENCODING`, and `@MODIFIER` parts are optional.
///
/// # Examples
///
/// - `en` - Just language
/// - `en_US` - Language and country
/// - `sr_YU@Latn` - Language, country, and modifier
/// - `en_US.UTF-8@euro` - All components
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Locale {
    /// Language code (e.g., "en", "fr", "sr")
    pub lang: String,
    /// Optional country code (e.g., "US", "GB", "YU")
    pub country: Option<String>,
    /// Optional encoding (e.g., "UTF-8"), usually ignored for matching
    pub encoding: Option<String>,
    /// Optional modifier (e.g., "Latn", "euro")
    pub modifier: Option<String>,
}

impl Locale {
    /// Creates a new Locale with just a language code.
    pub fn new(lang: impl Into<String>) -> Self {
        Self {
            lang: lang.into(),
            country: None,
            encoding: None,
            modifier: None,
        }
    }

    /// Creates a locale from a string like "en_US.UTF-8@euro".
    ///
    /// # Examples
    ///
    /// ```
    /// use xdg_desktop_entry::Locale;
    ///
    /// let locale = Locale::from_string("sr_YU@Latn");
    /// assert_eq!(locale.lang, "sr");
    /// assert_eq!(locale.country, Some("YU".to_string()));
    /// assert_eq!(locale.modifier, Some("Latn".to_string()));
    ///
    /// let locale2 = Locale::from_string("en_US.UTF-8");
    /// assert_eq!(locale2.lang, "en");
    /// assert_eq!(locale2.country, Some("US".to_string()));
    /// assert_eq!(locale2.encoding, Some("UTF-8".to_string()));
    /// ```
    pub fn from_string(s: &str) -> Self {
        let mut locale = Self {
            lang: String::new(),
            country: None,
            encoding: None,
            modifier: None,
        };

        // Parse modifier first (after @)
        let (base, modifier) = if let Some(at_pos) = s.rfind('@') {
            locale.modifier = Some(s[at_pos + 1..].to_string());
            (&s[..at_pos], true)
        } else {
            (s, false)
        };

        // Parse encoding (after .)
        let (base, _has_encoding) = if !modifier && base.contains('.') {
            if let Some(dot_pos) = base.rfind('.') {
                locale.encoding = Some(base[dot_pos + 1..].to_string());
                (&base[..dot_pos], true)
            } else {
                (base, false)
            }
        } else if modifier {
            // Could still have encoding before modifier
            if let Some(dot_pos) = base.rfind('.') {
                locale.encoding = Some(base[dot_pos + 1..].to_string());
                (&base[..dot_pos], true)
            } else {
                (base, false)
            }
        } else {
            (base, false)
        };

        // Parse country (after _)
        if let Some(underscore_pos) = base.find('_') {
            locale.lang = base[..underscore_pos].to_string();
            locale.country = Some(base[underscore_pos + 1..].to_string());
        } else {
            locale.lang = base.to_string();
        }

        locale
    }

    /// Converts the locale to its string representation.
    pub fn to_string_repr(&self) -> String {
        let mut result = self.lang.clone();
        if let Some(country) = &self.country {
            result.push('_');
            result.push_str(country);
        }
        if let Some(encoding) = &self.encoding {
            result.push('.');
            result.push_str(encoding);
        }
        if let Some(modifier) = &self.modifier {
            result.push('@');
            result.push_str(modifier);
        }
        result
    }
}

// ============================================================================
// Localized Values
// ============================================================================

/// Represents a localizable string value.
///
/// Desktop entries support localization by allowing keys to have locale-specific
/// variants (e.g., `Name[fr]=...`). This struct stores the default value and all
/// localized variants.
///
/// # Specification Reference
///
/// Section 5: "Localized values for keys"
#[derive(Debug, Clone, PartialEq)]
pub struct LocalizedString {
    /// The default value (key without locale suffix)
    pub default: String,
    /// Map of locale to localized value
    pub localized: HashMap<Locale, String>,
}

impl LocalizedString {
    /// Creates a new LocalizedString with just a default value.
    pub fn new(default: impl Into<String>) -> Self {
        Self {
            default: default.into(),
            localized: HashMap::new(),
        }
    }

    /// Adds a localized variant.
    pub fn add_localized(&mut self, locale: Locale, value: String) {
        self.localized.insert(locale, value);
    }

    /// Gets the appropriate value for the given locale using the spec's matching rules.
    ///
    /// # Matching Rules (Section 5)
    ///
    /// 1. Try exact match: `lang_COUNTRY@MODIFIER`
    /// 2. Try without country: `lang@MODIFIER`
    /// 3. Try without modifier: `lang_COUNTRY`
    /// 4. Try just language: `lang`
    /// 5. Fall back to default
    pub fn get(&self, locale: &Locale) -> &str {
        // 1. Try exact match
        if let Some(value) = self.localized.get(locale) {
            return value;
        }

        // 2. Try without country (lang@MODIFIER)
        if locale.country.is_some() && locale.modifier.is_some() {
            let mut try_locale = locale.clone();
            try_locale.country = None;
            if let Some(value) = self.localized.get(&try_locale) {
                return value;
            }
        }

        // 3. Try without modifier (lang_COUNTRY)
        if locale.modifier.is_some() {
            let mut try_locale = locale.clone();
            try_locale.modifier = None;
            if let Some(value) = self.localized.get(&try_locale) {
                return value;
            }
        }

        // 4. Try just language
        if locale.country.is_some() || locale.modifier.is_some() {
            let try_locale = Locale::new(&locale.lang);
            if let Some(value) = self.localized.get(&try_locale) {
                return value;
            }
        }

        // 5. Fall back to default
        &self.default
    }
}

/// Represents an icon name or path, which can also be localized.
///
/// Icon values can be either:
/// - Absolute paths to icon files
/// - Icon names to be looked up via the Icon Theme Specification
///
/// # Specification Reference
///
/// Section 4: "Values of type `iconstring`"
/// Section 6: "`Icon` key"
#[derive(Debug, Clone, PartialEq)]
pub struct IconString {
    /// The default icon name or path
    pub default: String,
    /// Map of locale to localized icon name or path
    pub localized: HashMap<Locale, String>,
}

impl IconString {
    /// Creates a new IconString with just a default value.
    pub fn new(default: impl Into<String>) -> Self {
        Self {
            default: default.into(),
            localized: HashMap::new(),
        }
    }

    /// Adds a localized variant.
    pub fn add_localized(&mut self, locale: Locale, value: String) {
        self.localized.insert(locale, value);
    }

    /// Gets the appropriate icon for the given locale.
    pub fn get(&self, locale: &Locale) -> &str {
        // Use the same matching logic as LocalizedString
        if let Some(value) = self.localized.get(locale) {
            return value;
        }

        if locale.country.is_some() && locale.modifier.is_some() {
            let mut try_locale = locale.clone();
            try_locale.country = None;
            if let Some(value) = self.localized.get(&try_locale) {
                return value;
            }
        }

        if locale.modifier.is_some() {
            let mut try_locale = locale.clone();
            try_locale.modifier = None;
            if let Some(value) = self.localized.get(&try_locale) {
                return value;
            }
        }

        if locale.country.is_some() || locale.modifier.is_some() {
            let try_locale = Locale::new(&locale.lang);
            if let Some(value) = self.localized.get(&try_locale) {
                return value;
            }
        }

        &self.default
    }
}

/// Represents a list of localized strings (e.g., Keywords).
///
/// Some keys like `Keywords` have type `localestring(s)`, meaning they can
/// contain multiple localized strings separated by semicolons.
///
/// # Specification Reference
///
/// Section 4: "Some keys can have multiple values"
#[derive(Debug, Clone, PartialEq)]
pub struct LocalizedStringList {
    /// The default list of values
    pub default: Vec<String>,
    /// Map of locale to localized list of values
    pub localized: HashMap<Locale, Vec<String>>,
}

impl LocalizedStringList {
    /// Creates a new LocalizedStringList with default values.
    pub fn new(default: Vec<String>) -> Self {
        Self {
            default,
            localized: HashMap::new(),
        }
    }

    /// Adds a localized variant.
    pub fn add_localized(&mut self, locale: Locale, values: Vec<String>) {
        self.localized.insert(locale, values);
    }

    /// Gets the appropriate list for the given locale.
    pub fn get(&self, locale: &Locale) -> &[String] {
        if let Some(value) = self.localized.get(locale) {
            return value;
        }

        if locale.country.is_some() && locale.modifier.is_some() {
            let mut try_locale = locale.clone();
            try_locale.country = None;
            if let Some(value) = self.localized.get(&try_locale) {
                return value;
            }
        }

        if locale.modifier.is_some() {
            let mut try_locale = locale.clone();
            try_locale.modifier = None;
            if let Some(value) = self.localized.get(&try_locale) {
                return value;
            }
        }

        if locale.country.is_some() || locale.modifier.is_some() {
            let try_locale = Locale::new(&locale.lang);
            if let Some(value) = self.localized.get(&try_locale) {
                return value;
            }
        }

        &self.default
    }
}

// ============================================================================
// Desktop Entry Types
// ============================================================================

/// The type of desktop entry.
///
/// # Specification Reference
///
/// Section 6: "`Type` key" - Defines 3 types of desktop entries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopEntryType {
    /// An application that can be launched (type 1)
    Application,
    /// A link to a URL (type 2)
    Link,
    /// A directory/folder (type 3)
    Directory,
}

impl DesktopEntryType {
    /// Parses a type string into a DesktopEntryType.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Application" => Some(Self::Application),
            "Link" => Some(Self::Link),
            "Directory" => Some(Self::Directory),
            _ => None,
        }
    }

    /// Converts the type to its string representation.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Application => "Application",
            Self::Link => "Link",
            Self::Directory => "Directory",
        }
    }
}

// ============================================================================
// Group and Entry
// ============================================================================

/// Represents a comment or blank line in the file.
#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    /// Line number in the original file
    pub line_number: usize,
    /// The comment text (without the # prefix) or empty for blank lines
    pub content: String,
    /// Whether this is a blank line (vs a comment)
    pub is_blank: bool,
}

/// Represents an additional group in a desktop file.
///
/// Desktop files can contain multiple groups. The main group is always
/// `[Desktop Entry]`, but there can be action groups like
/// `[Desktop Action new-window]` or custom extension groups.
///
/// # Specification Reference
///
/// Section 3.2: "Group headers"
/// Section 11: "Additional applications actions"
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    /// Name of the group (without the brackets)
    pub name: String,
    /// All key-value pairs in this group
    pub entries: HashMap<String, Vec<Entry>>,
}

/// Represents a single key-value entry, which may be localized.
///
/// # Specification Reference
///
/// Section 3.3: "Entries"
/// Section 5: "Localized values for keys"
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The key name (without locale suffix)
    pub key: String,
    /// The locale for this entry (None for the default)
    pub locale: Option<Locale>,
    /// The raw value as a string
    pub value: String,
}

// ============================================================================
// Desktop Entry
// ============================================================================

/// Represents a complete desktop entry file.
///
/// This struct contains all the standard keys defined in the Desktop Entry
/// Specification. Keys that only apply to certain types are optional and
/// should only be set when appropriate.
///
/// # Required Keys
///
/// - `entry_type`: Always required
/// - `name`: Always required
/// - `url`: Required only when `entry_type` is `Link`
///
/// # Specification Reference
///
/// Section 6: "Recognized desktop entry keys", Table 2
#[derive(Debug, Clone)]
pub struct DesktopEntry {
    // ============================================================
    // Required Keys (context-dependent)
    // ============================================================
    /// Type of desktop entry: Application, Link, or Directory.
    ///
    /// **Required:** Always
    pub entry_type: DesktopEntryType,

    /// Specific name of the application (e.g., "Mozilla").
    ///
    /// **Required:** Always
    /// **Type:** localestring
    pub name: LocalizedString,

    /// URL to access (e.g., "<https://example.com>").
    ///
    /// **Required:** Only when `entry_type` is `Link`
    /// **Type:** string
    pub url: Option<String>,

    // ============================================================
    // Optional Keys (common to all types: 1-3)
    // ============================================================
    /// Version of the Desktop Entry Specification (e.g., "1.5").
    ///
    /// **Type:** string
    /// **Applies to:** All types (1-3)
    pub version: Option<String>,

    /// Generic name of the application (e.g., "Web Browser").
    ///
    /// **Type:** localestring
    /// **Applies to:** All types (1-3)
    pub generic_name: Option<LocalizedString>,

    /// Whether to hide this entry from menus.
    ///
    /// If `true`, the application exists but shouldn't be displayed in menus.
    /// Useful for MIME type associations without menu entries.
    ///
    /// **Type:** boolean
    /// **Applies to:** All types (1-3)
    pub no_display: Option<bool>,

    /// Tooltip/description for the entry (e.g., "View sites on the Internet").
    ///
    /// Should not be redundant with `name` or `generic_name`.
    ///
    /// **Type:** localestring
    /// **Applies to:** All types (1-3)
    pub comment: Option<LocalizedString>,

    /// Icon to display in file managers, menus, etc.
    ///
    /// Can be an absolute path or an icon name for theme lookup.
    ///
    /// **Type:** iconstring
    /// **Applies to:** All types (1-3)
    pub icon: Option<IconString>,

    /// Whether this entry has been deleted by the user.
    ///
    /// If `true`, treat as if the file doesn't exist.
    ///
    /// **Type:** boolean
    /// **Applies to:** All types (1-3)
    pub hidden: Option<bool>,

    /// Desktop environments that should display this entry.
    ///
    /// If present, only show in these environments (matched against `$XDG_CURRENT_DESKTOP`).
    ///
    /// **Type:** string(s)
    /// **Applies to:** All types (1-3)
    pub only_show_in: Option<Vec<String>>,

    /// Desktop environments that should NOT display this entry.
    ///
    /// **Type:** string(s)
    /// **Applies to:** All types (1-3)
    pub not_show_in: Option<Vec<String>>,

    /// Whether D-Bus activation is supported.
    ///
    /// If `true`, use D-Bus to launch instead of the `Exec` key.
    ///
    /// **Type:** boolean
    /// **Applies to:** Application (type 1)
    pub dbus_activatable: Option<bool>,

    // ============================================================
    // Application-Specific Keys (type 1 only)
    // ============================================================
    /// Path to check if the program is installed.
    ///
    /// If the file doesn't exist or isn't executable, the entry may be ignored.
    ///
    /// **Type:** string
    /// **Applies to:** Application (type 1)
    pub try_exec: Option<String>,

    /// Command to execute, possibly with arguments and field codes.
    ///
    /// Required if `dbus_activatable` is not `true`.
    /// See Section 7 for field code details (`%f`, `%u`, etc.).
    ///
    /// **Type:** string
    /// **Applies to:** Application (type 1)
    pub exec: Option<String>,

    /// Working directory for the program.
    ///
    /// **Type:** string
    /// **Applies to:** Application (type 1)
    pub path: Option<String>,

    /// Whether the program runs in a terminal.
    ///
    /// **Type:** boolean
    /// **Applies to:** Application (type 1)
    pub terminal: Option<bool>,

    /// Additional actions this application supports.
    ///
    /// References action groups defined later in the file (e.g., `[Desktop Action new-window]`).
    ///
    /// **Type:** string(s)
    /// **Applies to:** Application (type 1)
    pub actions: Option<Vec<String>>,

    /// MIME types supported by this application.
    ///
    /// **Type:** string(s)
    /// **Applies to:** Application (type 1)
    pub mime_type: Option<Vec<String>>,

    /// Categories for menu placement.
    ///
    /// See Desktop Menu Specification for valid values.
    ///
    /// **Type:** string(s)
    /// **Applies to:** Application (type 1)
    pub categories: Option<Vec<String>>,

    /// Interfaces this application implements.
    ///
    /// See Section 9 for details on interface declarations.
    ///
    /// **Type:** string(s)
    /// **Applies to:** Application (type 1)
    pub implements: Option<Vec<String>>,

    /// Keywords for searching (not for display).
    ///
    /// Should not duplicate `name` or `generic_name`.
    ///
    /// **Type:** localestring(s)
    /// **Applies to:** Application (type 1)
    pub keywords: Option<LocalizedStringList>,

    /// Whether the app sends startup notification messages.
    ///
    /// **Type:** boolean
    /// **Applies to:** Application (type 1)
    pub startup_notify: Option<bool>,

    /// WM class or name hint for startup notification.
    ///
    /// **Type:** string
    /// **Applies to:** Application (type 1)
    pub startup_wm_class: Option<String>,

    /// Whether the app prefers a discrete GPU.
    ///
    /// Hint only; support depends on implementation.
    ///
    /// **Type:** boolean
    /// **Applies to:** Application (type 1)
    pub prefers_non_default_gpu: Option<bool>,

    /// Whether the app has a single main window.
    ///
    /// Hint to avoid offering UI to open additional windows.
    ///
    /// **Type:** boolean
    /// **Applies to:** Application (type 1)
    pub single_main_window: Option<bool>,

    // ============================================================
    // Additional Groups
    // ============================================================
    /// Additional groups in the desktop file (e.g., action groups, custom extensions).
    ///
    /// The main `[Desktop Entry]` group is represented by the fields above.
    /// This field stores any other groups like `[Desktop Action ...]`.
    pub additional_groups: HashMap<String, Group>,

    // ============================================================
    // Raw Data (for round-trip support)
    // ============================================================
    /// Unrecognized keys in the main Desktop Entry group (preserved for round-trip)
    pub unknown_keys: HashMap<String, Vec<Entry>>,

    /// Comments and blank lines (preserved for round-trip serialization)
    pub comments: Vec<Comment>,
}

impl DesktopEntry {
    /// Creates a new minimal DesktopEntry with required fields.
    ///
    /// # Examples
    ///
    /// ```
    /// use xdg_desktop_entry::{DesktopEntry, DesktopEntryType, LocalizedString};
    ///
    /// let entry = DesktopEntry::new(
    ///     DesktopEntryType::Application,
    ///     LocalizedString::new("My Application"),
    /// );
    /// ```
    pub fn new(entry_type: DesktopEntryType, name: LocalizedString) -> Self {
        Self {
            entry_type,
            name,
            url: None,
            version: None,
            generic_name: None,
            no_display: None,
            comment: None,
            icon: None,
            hidden: None,
            only_show_in: None,
            not_show_in: None,
            dbus_activatable: None,
            try_exec: None,
            exec: None,
            path: None,
            terminal: None,
            actions: None,
            mime_type: None,
            categories: None,
            implements: None,
            keywords: None,
            startup_notify: None,
            startup_wm_class: None,
            prefers_non_default_gpu: None,
            single_main_window: None,
            additional_groups: HashMap::new(),
            unknown_keys: HashMap::new(),
            comments: Vec::new(),
        }
    }

    /// Parses a desktop entry file from a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use xdg_desktop_entry::DesktopEntry;
    ///
    /// let content = r#"[Desktop Entry]
    /// Type=Application
    /// Name=Test App
    /// Exec=test-app
    /// "#;
    ///
    /// let entry = DesktopEntry::parse(content).unwrap();
    /// assert_eq!(entry.name.default, "Test App");
    /// ```
    pub fn parse(content: &str) -> Result<Self> {
        Parser::new(content).parse()
    }

    /// Parses a desktop entry file from a file path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use xdg_desktop_entry::DesktopEntry;
    ///
    /// let entry = DesktopEntry::parse_file("app.desktop").unwrap();
    /// ```
    pub fn parse_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Serializes the desktop entry to a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use xdg_desktop_entry::{DesktopEntry, DesktopEntryType, LocalizedString};
    ///
    /// let mut entry = DesktopEntry::new(
    ///     DesktopEntryType::Application,
    ///     LocalizedString::new("My App"),
    /// );
    /// entry.exec = Some("my-app".to_string());
    ///
    /// let serialized = entry.serialize();
    /// assert!(serialized.contains("[Desktop Entry]"));
    /// assert!(serialized.contains("Type=Application"));
    /// ```
    pub fn serialize(&self) -> String {
        let mut output = Vec::new();
        self.write_to(&mut output).unwrap();
        String::from_utf8(output).unwrap()
    }

    /// Writes the desktop entry to a writer.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // Write comments at the beginning
        for comment in &self.comments {
            if comment.is_blank {
                writeln!(writer)?;
            } else {
                writeln!(writer, "#{}", comment.content)?;
            }
        }

        // Write [Desktop Entry] group
        writeln!(writer, "[Desktop Entry]")?;

        // Type (required)
        writeln!(writer, "Type={}", self.entry_type.as_str())?;

        // Version (optional)
        if let Some(version) = &self.version {
            writeln!(writer, "Version={}", version)?;
        }

        // Name (required)
        writeln!(writer, "Name={}", self.name.default)?;
        for (locale, value) in &self.name.localized {
            writeln!(writer, "Name[{}]={}", locale.to_string_repr(), value)?;
        }

        // GenericName
        if let Some(generic_name) = &self.generic_name {
            writeln!(writer, "GenericName={}", generic_name.default)?;
            for (locale, value) in &generic_name.localized {
                writeln!(writer, "GenericName[{}]={}", locale.to_string_repr(), value)?;
            }
        }

        // NoDisplay
        if let Some(no_display) = self.no_display {
            writeln!(writer, "NoDisplay={}", no_display)?;
        }

        // Comment
        if let Some(comment) = &self.comment {
            writeln!(writer, "Comment={}", comment.default)?;
            for (locale, value) in &comment.localized {
                writeln!(writer, "Comment[{}]={}", locale.to_string_repr(), value)?;
            }
        }

        // Icon
        if let Some(icon) = &self.icon {
            writeln!(writer, "Icon={}", icon.default)?;
            for (locale, value) in &icon.localized {
                writeln!(writer, "Icon[{}]={}", locale.to_string_repr(), value)?;
            }
        }

        // Hidden
        if let Some(hidden) = self.hidden {
            writeln!(writer, "Hidden={}", hidden)?;
        }

        // OnlyShowIn
        if let Some(only_show_in) = &self.only_show_in {
            writeln!(writer, "OnlyShowIn={}", only_show_in.join(";"))?;
        }

        // NotShowIn
        if let Some(not_show_in) = &self.not_show_in {
            writeln!(writer, "NotShowIn={}", not_show_in.join(";"))?;
        }

        // DBusActivatable
        if let Some(dbus_activatable) = self.dbus_activatable {
            writeln!(writer, "DBusActivatable={}", dbus_activatable)?;
        }

        // TryExec
        if let Some(try_exec) = &self.try_exec {
            writeln!(writer, "TryExec={}", try_exec)?;
        }

        // Exec
        if let Some(exec) = &self.exec {
            writeln!(writer, "Exec={}", exec)?;
        }

        // Path
        if let Some(path) = &self.path {
            writeln!(writer, "Path={}", path)?;
        }

        // Terminal
        if let Some(terminal) = self.terminal {
            writeln!(writer, "Terminal={}", terminal)?;
        }

        // Actions
        if let Some(actions) = &self.actions {
            writeln!(writer, "Actions={}", actions.join(";"))?;
        }

        // MimeType
        if let Some(mime_type) = &self.mime_type {
            writeln!(writer, "MimeType={}", mime_type.join(";"))?;
        }

        // Categories
        if let Some(categories) = &self.categories {
            writeln!(writer, "Categories={}", categories.join(";"))?;
        }

        // Implements
        if let Some(implements) = &self.implements {
            writeln!(writer, "Implements={}", implements.join(";"))?;
        }

        // Keywords
        if let Some(keywords) = &self.keywords {
            writeln!(writer, "Keywords={}", keywords.default.join(";"))?;
            for (locale, values) in &keywords.localized {
                writeln!(
                    writer,
                    "Keywords[{}]={}",
                    locale.to_string_repr(),
                    values.join(";")
                )?;
            }
        }

        // StartupNotify
        if let Some(startup_notify) = self.startup_notify {
            writeln!(writer, "StartupNotify={}", startup_notify)?;
        }

        // StartupWMClass
        if let Some(startup_wm_class) = &self.startup_wm_class {
            writeln!(writer, "StartupWMClass={}", startup_wm_class)?;
        }

        // URL (for Link type)
        if let Some(url) = &self.url {
            writeln!(writer, "URL={}", url)?;
        }

        // PrefersNonDefaultGPU
        if let Some(prefers_non_default_gpu) = self.prefers_non_default_gpu {
            writeln!(writer, "PrefersNonDefaultGPU={}", prefers_non_default_gpu)?;
        }

        // SingleMainWindow
        if let Some(single_main_window) = self.single_main_window {
            writeln!(writer, "SingleMainWindow={}", single_main_window)?;
        }

        // Unknown keys (for round-trip)
        for (key, entries) in &self.unknown_keys {
            for entry in entries {
                if let Some(locale) = &entry.locale {
                    writeln!(
                        writer,
                        "{}[{}]={}",
                        key,
                        locale.to_string_repr(),
                        entry.value
                    )?;
                } else {
                    writeln!(writer, "{}={}", key, entry.value)?;
                }
            }
        }

        // Additional groups
        for (_, group) in &self.additional_groups {
            writeln!(writer)?;
            writeln!(writer, "[{}]", group.name)?;
            for (key, entries) in &group.entries {
                for entry in entries {
                    if let Some(locale) = &entry.locale {
                        writeln!(
                            writer,
                            "{}[{}]={}",
                            key,
                            locale.to_string_repr(),
                            entry.value
                        )?;
                    } else {
                        writeln!(writer, "{}={}", key, entry.value)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Validates that required fields are present for the entry type.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    pub fn validate(&self) -> Result<()> {
        // URL is required for Link type
        if self.entry_type == DesktopEntryType::Link && self.url.is_none() {
            return Err(DesktopEntryError::ValidationError(
                "URL is required for Link type entries".to_string(),
            ));
        }

        // Exec or DBusActivatable is required for Application type
        if self.entry_type == DesktopEntryType::Application {
            let has_exec = self.exec.is_some();
            let is_dbus_activatable = self.dbus_activatable.unwrap_or(false);

            if !has_exec && !is_dbus_activatable {
                return Err(DesktopEntryError::ValidationError(
                    "Either Exec key or DBusActivatable=true is required for Application type"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }
}

// ============================================================================
// Parser
// ============================================================================

struct Parser {
    lines: Vec<String>,
}

impl Parser {
    fn new(content: &str) -> Self {
        Self {
            lines: content.lines().map(|s| s.to_string()).collect(),
        }
    }

    fn parse(&mut self) -> Result<DesktopEntry> {
        let mut groups: HashMap<String, HashMap<String, Vec<Entry>>> = HashMap::new();
        let mut current_group: Option<String> = None;
        let mut comments = Vec::new();
        let mut line_num = 0;

        // Parse all lines
        for line in &self.lines {
            line_num += 1;
            let trimmed = line.trim();

            // Skip blank lines and comments before first group
            if trimmed.is_empty() {
                if current_group.is_none() {
                    comments.push(Comment {
                        line_number: line_num,
                        content: String::new(),
                        is_blank: true,
                    });
                }
                continue;
            }

            if trimmed.starts_with('#') {
                if current_group.is_none() {
                    comments.push(Comment {
                        line_number: line_num,
                        content: trimmed[1..].to_string(),
                        is_blank: false,
                    });
                }
                continue;
            }

            // Group header
            if trimmed.starts_with('[') {
                if !trimmed.ends_with(']') {
                    return Err(DesktopEntryError::InvalidGroupHeader(
                        line_num,
                        line.clone(),
                    ));
                }

                let group_name = trimmed[1..trimmed.len() - 1].to_string();

                // Check for duplicate groups
                if groups.contains_key(&group_name) {
                    return Err(DesktopEntryError::DuplicateGroup(group_name));
                }

                groups.insert(group_name.clone(), HashMap::new());
                current_group = Some(group_name);
                continue;
            }

            // Key-value pair
            if let Some(eq_pos) = line.find('=') {
                let key_part = &line[..eq_pos];
                let value = &line[eq_pos + 1..];

                // Parse key and locale
                let (key, locale) = if let Some(bracket_start) = key_part.find('[') {
                    if let Some(bracket_end) = key_part.find(']') {
                        let key = key_part[..bracket_start].trim().to_string();
                        let locale_str = &key_part[bracket_start + 1..bracket_end];
                        (key, Some(Locale::from_string(locale_str)))
                    } else {
                        return Err(DesktopEntryError::InvalidLine(line_num, line.clone()));
                    }
                } else {
                    (key_part.trim().to_string(), None)
                };

                // Validate key name (spec: only A-Za-z0-9-)
                if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                    return Err(DesktopEntryError::InvalidKeyName(line_num, key.clone()));
                }

                // Add to current group
                if let Some(group_name) = &current_group {
                    let group = groups.get_mut(group_name).unwrap();
                    let entry = Entry {
                        key: key.clone(),
                        locale,
                        value: value.to_string(),
                    };
                    group.entry(key).or_insert_with(Vec::new).push(entry);
                } else {
                    return Err(DesktopEntryError::InvalidLine(line_num, line.clone()));
                }
            } else {
                return Err(DesktopEntryError::InvalidLine(line_num, line.clone()));
            }
        }

        // Must have Desktop Entry group
        let desktop_entry_data = groups
            .remove("Desktop Entry")
            .ok_or(DesktopEntryError::MissingDesktopEntryGroup)?;

        // Parse Type (required)
        let type_entries = desktop_entry_data
            .get("Type")
            .and_then(|v| v.first())
            .ok_or_else(|| DesktopEntryError::MissingRequiredKey("Type".to_string()))?;

        let entry_type = DesktopEntryType::from_str(&type_entries.value).ok_or_else(|| {
            DesktopEntryError::InvalidValue("Type".to_string(), type_entries.value.clone())
        })?;

        // Parse Name (required)
        let name_entries = desktop_entry_data
            .get("Name")
            .ok_or_else(|| DesktopEntryError::MissingRequiredKey("Name".to_string()))?;

        let mut name = LocalizedString::new("");
        for entry in name_entries {
            if let Some(locale) = &entry.locale {
                name.localized.insert(locale.clone(), entry.value.clone());
            } else {
                name.default = entry.value.clone();
            }
        }

        // Create desktop entry
        let mut desktop_entry = DesktopEntry::new(entry_type, name);
        desktop_entry.comments = comments;

        // Parse optional fields
        Self::parse_optional_string(&desktop_entry_data, "Version", &mut desktop_entry.version);
        Self::parse_optional_localized_string(
            &desktop_entry_data,
            "GenericName",
            &mut desktop_entry.generic_name,
        );
        Self::parse_optional_bool(
            &desktop_entry_data,
            "NoDisplay",
            &mut desktop_entry.no_display,
        );
        Self::parse_optional_localized_string(
            &desktop_entry_data,
            "Comment",
            &mut desktop_entry.comment,
        );
        Self::parse_optional_icon_string(&desktop_entry_data, "Icon", &mut desktop_entry.icon);
        Self::parse_optional_bool(&desktop_entry_data, "Hidden", &mut desktop_entry.hidden);
        Self::parse_optional_string_list(
            &desktop_entry_data,
            "OnlyShowIn",
            &mut desktop_entry.only_show_in,
        );
        Self::parse_optional_string_list(
            &desktop_entry_data,
            "NotShowIn",
            &mut desktop_entry.not_show_in,
        );
        Self::parse_optional_bool(
            &desktop_entry_data,
            "DBusActivatable",
            &mut desktop_entry.dbus_activatable,
        );
        Self::parse_optional_string(&desktop_entry_data, "TryExec", &mut desktop_entry.try_exec);
        Self::parse_optional_string(&desktop_entry_data, "Exec", &mut desktop_entry.exec);
        Self::parse_optional_string(&desktop_entry_data, "Path", &mut desktop_entry.path);
        Self::parse_optional_bool(&desktop_entry_data, "Terminal", &mut desktop_entry.terminal);
        Self::parse_optional_string_list(
            &desktop_entry_data,
            "Actions",
            &mut desktop_entry.actions,
        );
        Self::parse_optional_string_list(
            &desktop_entry_data,
            "MimeType",
            &mut desktop_entry.mime_type,
        );
        Self::parse_optional_string_list(
            &desktop_entry_data,
            "Categories",
            &mut desktop_entry.categories,
        );
        Self::parse_optional_string_list(
            &desktop_entry_data,
            "Implements",
            &mut desktop_entry.implements,
        );
        Self::parse_optional_localized_string_list(
            &desktop_entry_data,
            "Keywords",
            &mut desktop_entry.keywords,
        );
        Self::parse_optional_bool(
            &desktop_entry_data,
            "StartupNotify",
            &mut desktop_entry.startup_notify,
        );
        Self::parse_optional_string(
            &desktop_entry_data,
            "StartupWMClass",
            &mut desktop_entry.startup_wm_class,
        );
        Self::parse_optional_string(&desktop_entry_data, "URL", &mut desktop_entry.url);
        Self::parse_optional_bool(
            &desktop_entry_data,
            "PrefersNonDefaultGPU",
            &mut desktop_entry.prefers_non_default_gpu,
        );
        Self::parse_optional_bool(
            &desktop_entry_data,
            "SingleMainWindow",
            &mut desktop_entry.single_main_window,
        );

        // Store unknown keys
        let known_keys = [
            "Type",
            "Name",
            "Version",
            "GenericName",
            "NoDisplay",
            "Comment",
            "Icon",
            "Hidden",
            "OnlyShowIn",
            "NotShowIn",
            "DBusActivatable",
            "TryExec",
            "Exec",
            "Path",
            "Terminal",
            "Actions",
            "MimeType",
            "Categories",
            "Implements",
            "Keywords",
            "StartupNotify",
            "StartupWMClass",
            "URL",
            "PrefersNonDefaultGPU",
            "SingleMainWindow",
        ];

        for (key, entries) in desktop_entry_data {
            if !known_keys.contains(&key.as_str()) {
                desktop_entry.unknown_keys.insert(key, entries);
            }
        }

        // Parse additional groups
        for (group_name, group_data) in groups {
            let group = Group {
                name: group_name.clone(),
                entries: group_data,
            };
            desktop_entry.additional_groups.insert(group_name, group);
        }

        Ok(desktop_entry)
    }

    fn parse_optional_string(
        data: &HashMap<String, Vec<Entry>>,
        key: &str,
        target: &mut Option<String>,
    ) {
        if let Some(entries) = data.get(key) {
            if let Some(entry) = entries.first() {
                *target = Some(entry.value.clone());
            }
        }
    }

    fn parse_optional_bool(
        data: &HashMap<String, Vec<Entry>>,
        key: &str,
        target: &mut Option<bool>,
    ) {
        if let Some(entries) = data.get(key) {
            if let Some(entry) = entries.first() {
                *target = match entry.value.as_str() {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => None,
                };
            }
        }
    }

    fn parse_optional_string_list(
        data: &HashMap<String, Vec<Entry>>,
        key: &str,
        target: &mut Option<Vec<String>>,
    ) {
        if let Some(entries) = data.get(key) {
            if let Some(entry) = entries.first() {
                let list: Vec<String> = entry
                    .value
                    .split(';')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
                if !list.is_empty() {
                    *target = Some(list);
                }
            }
        }
    }

    fn parse_optional_localized_string(
        data: &HashMap<String, Vec<Entry>>,
        key: &str,
        target: &mut Option<LocalizedString>,
    ) {
        if let Some(entries) = data.get(key) {
            let mut localized = LocalizedString::new("");
            for entry in entries {
                if let Some(locale) = &entry.locale {
                    localized
                        .localized
                        .insert(locale.clone(), entry.value.clone());
                } else {
                    localized.default = entry.value.clone();
                }
            }
            *target = Some(localized);
        }
    }

    fn parse_optional_icon_string(
        data: &HashMap<String, Vec<Entry>>,
        key: &str,
        target: &mut Option<IconString>,
    ) {
        if let Some(entries) = data.get(key) {
            let mut icon = IconString::new("");
            for entry in entries {
                if let Some(locale) = &entry.locale {
                    icon.localized.insert(locale.clone(), entry.value.clone());
                } else {
                    icon.default = entry.value.clone();
                }
            }
            *target = Some(icon);
        }
    }

    fn parse_optional_localized_string_list(
        data: &HashMap<String, Vec<Entry>>,
        key: &str,
        target: &mut Option<LocalizedStringList>,
    ) {
        if let Some(entries) = data.get(key) {
            let mut list = LocalizedStringList::new(Vec::new());
            for entry in entries {
                let values: Vec<String> = entry
                    .value
                    .split(';')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();

                if let Some(locale) = &entry.locale {
                    list.localized.insert(locale.clone(), values);
                } else {
                    list.default = values;
                }
            }
            *target = Some(list);
        }
    }
}
