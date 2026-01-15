//! # XDG Desktop Entry Parser
//!
//! This library provides type-safe data structures for parsing and representing
//! XDG Desktop Entry files according to the [Desktop Entry Specification 1.5].
//!
//! Desktop entries are used by desktop environments to describe applications,
//! links, and directories in a standardized format.
//!
//! [Desktop Entry Specification 1.5]: https://specifications.freedesktop.org/desktop-entry-spec/1.5/

use std::collections::HashMap;

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
    /// ```ignore
    /// let locale = Locale::from_str("sr_YU@Latn");
    /// assert_eq!(locale.lang, "sr");
    /// assert_eq!(locale.country, Some("YU".to_string()));
    /// assert_eq!(locale.modifier, Some("Latn".to_string()));
    /// ```
    pub fn from_string(s: &str) -> Self {
        // This is a placeholder - actual parsing would be implemented in the parser
        Self::new(s)
    }
}

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
        // This is a placeholder - actual matching logic would be more complex
        self.localized
            .get(locale)
            .map(|s| s.as_str())
            .unwrap_or(&self.default)
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

    /// Gets the appropriate icon for the given locale.
    pub fn get(&self, locale: &Locale) -> &str {
        self.localized
            .get(locale)
            .map(|s| s.as_str())
            .unwrap_or(&self.default)
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

    /// Gets the appropriate list for the given locale.
    pub fn get(&self, locale: &Locale) -> &[String] {
        self.localized
            .get(locale)
            .map(|v| v.as_slice())
            .unwrap_or(&self.default)
    }
}

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

impl DesktopEntry {
    /// Creates a new minimal DesktopEntry with required fields.
    ///
    /// # Examples
    ///
    /// ```ignore
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
        }
    }

    /// Validates that required fields are present for the entry type.
    ///
    /// # Errors
    ///
    /// Returns an error message if validation fails.
    pub fn validate(&self) -> Result<(), String> {
        // URL is required for Link type
        if self.entry_type == DesktopEntryType::Link && self.url.is_none() {
            return Err("URL is required for Link type entries".to_string());
        }

        // Exec or DBusActivatable is required for Application type
        if self.entry_type == DesktopEntryType::Application {
            let has_exec = self.exec.is_some();
            let is_dbus_activatable = self.dbus_activatable.unwrap_or(false);

            if !has_exec && !is_dbus_activatable {
                return Err(
                    "Either Exec key or DBusActivatable=true is required for Application type"
                        .to_string(),
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_creation() {
        let locale = Locale::new("en");
        assert_eq!(locale.lang, "en");
        assert_eq!(locale.country, None);
    }

    #[test]
    fn test_localized_string() {
        let mut ls = LocalizedString::new("Hello");
        ls.localized
            .insert(Locale::new("fr"), "Bonjour".to_string());

        assert_eq!(ls.default, "Hello");
        assert_eq!(ls.localized.get(&Locale::new("fr")).unwrap(), "Bonjour");
    }

    #[test]
    fn test_desktop_entry_validation() {
        // Valid application
        let mut app = DesktopEntry::new(
            DesktopEntryType::Application,
            LocalizedString::new("Test App"),
        );
        app.exec = Some("test-app".to_string());
        assert!(app.validate().is_ok());

        // Invalid link (missing URL)
        let link = DesktopEntry::new(DesktopEntryType::Link, LocalizedString::new("Test Link"));
        assert!(link.validate().is_err());

        // Valid link with URL
        let mut link = DesktopEntry::new(DesktopEntryType::Link, LocalizedString::new("Test Link"));
        link.url = Some("https://example.com".to_string());
        assert!(link.validate().is_ok());
    }
}
