use xdg_desktop_entry::{DesktopEntry, DesktopEntryError, DesktopEntryType, Locale};

#[test]
fn test_parse_minimal() {
    let entry = DesktopEntry::parse_file("tests/fixtures/valid/minimal.desktop").unwrap();

    assert_eq!(entry.entry_type, DesktopEntryType::Application);
    assert_eq!(entry.name.default, "Minimal App");
    assert!(entry.exec.is_some());
    assert_eq!(entry.exec.as_ref().unwrap(), "minimal-app");
}

#[test]
fn test_parse_full_entry() {
    let entry = DesktopEntry::parse_file("tests/fixtures/valid/full_entry.desktop").unwrap();

    assert_eq!(entry.entry_type, DesktopEntryType::Application);
    assert_eq!(entry.name.default, "Full Featured Application");
    assert_eq!(entry.version, Some("1.0".to_string()));

    // Check localized names
    assert_eq!(
        entry
            .name
            .localized
            .get(&Locale::from_string("es"))
            .unwrap(),
        "Aplicación Completa"
    );
    assert_eq!(
        entry
            .name
            .localized
            .get(&Locale::from_string("fr"))
            .unwrap(),
        "Application Complète"
    );

    // Check generic name
    assert!(entry.generic_name.is_some());
    assert_eq!(entry.generic_name.as_ref().unwrap().default, "Text Editor");

    // Check comment
    assert!(entry.comment.is_some());

    // Check icon
    assert!(entry.icon.is_some());
    assert_eq!(entry.icon.as_ref().unwrap().default, "text-editor");

    // Check exec
    assert_eq!(entry.exec, Some("full-app %F".to_string()));

    // Check path
    assert_eq!(entry.path, Some("/usr/share/full-app".to_string()));

    // Check terminal
    assert_eq!(entry.terminal, Some(false));

    // Check categories
    assert!(entry.categories.is_some());
    let categories = entry.categories.as_ref().unwrap();
    assert!(categories.contains(&"Utility".to_string()));
    assert!(categories.contains(&"TextEditor".to_string()));

    // Check MIME types
    assert!(entry.mime_type.is_some());
    let mime_types = entry.mime_type.as_ref().unwrap();
    assert!(mime_types.contains(&"text/plain".to_string()));

    // Check keywords
    assert!(entry.keywords.is_some());

    // Check startup notify
    assert_eq!(entry.startup_notify, Some(true));

    // Check startup WM class
    assert_eq!(entry.startup_wm_class, Some("FullApp".to_string()));

    // Check actions
    assert!(entry.actions.is_some());
    let actions = entry.actions.as_ref().unwrap();
    assert!(actions.contains(&"new-window".to_string()));
    assert!(actions.contains(&"preferences".to_string()));

    // Check additional groups (actions)
    assert!(
        entry
            .additional_groups
            .contains_key("Desktop Action new-window")
    );
    assert!(
        entry
            .additional_groups
            .contains_key("Desktop Action preferences")
    );
}

#[test]
fn test_parse_with_comments() {
    let entry = DesktopEntry::parse_file("tests/fixtures/valid/with_comments.desktop").unwrap();

    assert_eq!(entry.entry_type, DesktopEntryType::Application);
    assert_eq!(entry.name.default, "App with Comments");

    // Check that comments were preserved
    assert!(!entry.comments.is_empty());

    // Check custom extension group
    assert!(entry.additional_groups.contains_key("X-Custom Extension"));
}

#[test]
fn test_parse_spec_example() {
    let entry = DesktopEntry::parse_file("tests/fixtures/valid/spec_example.desktop").unwrap();

    assert_eq!(entry.entry_type, DesktopEntryType::Application);
    assert_eq!(entry.name.default, "Foo Viewer");
    assert_eq!(entry.version, Some("1.0".to_string()));
    assert!(entry.try_exec.is_some());

    // Check actions
    assert!(entry.actions.is_some());
    let actions = entry.actions.as_ref().unwrap();
    assert_eq!(actions.len(), 2);

    // Validate
    assert!(entry.validate().is_ok());
}

#[test]
fn test_missing_desktop_entry_group() {
    let result = DesktopEntry::parse_file("tests/fixtures/invalid/missing_desktop_entry.desktop");

    assert!(result.is_err());
    match result {
        Err(DesktopEntryError::MissingDesktopEntryGroup) => {}
        _ => panic!("Expected MissingDesktopEntryGroup error"),
    }
}

#[test]
fn test_duplicate_groups() {
    let result = DesktopEntry::parse_file("tests/fixtures/invalid/duplicate_groups.desktop");

    assert!(result.is_err());
    match result {
        Err(DesktopEntryError::DuplicateGroup(_)) => {}
        _ => panic!("Expected DuplicateGroup error"),
    }
}

#[test]
fn test_invalid_key_name() {
    let result = DesktopEntry::parse_file("tests/fixtures/invalid/invalid_key_name.desktop");

    assert!(result.is_err());
    match result {
        Err(DesktopEntryError::InvalidKeyName(_, _)) => {}
        _ => panic!("Expected InvalidKeyName error"),
    }
}

#[test]
fn test_locale_parsing() {
    // Just language
    let locale = Locale::from_string("en");
    assert_eq!(locale.lang, "en");
    assert_eq!(locale.country, None);
    assert_eq!(locale.encoding, None);
    assert_eq!(locale.modifier, None);

    // Language and country
    let locale = Locale::from_string("en_US");
    assert_eq!(locale.lang, "en");
    assert_eq!(locale.country, Some("US".to_string()));

    // Language, country, and encoding
    let locale = Locale::from_string("en_US.UTF-8");
    assert_eq!(locale.lang, "en");
    assert_eq!(locale.country, Some("US".to_string()));
    assert_eq!(locale.encoding, Some("UTF-8".to_string()));

    // Language, country, and modifier
    let locale = Locale::from_string("sr_YU@Latn");
    assert_eq!(locale.lang, "sr");
    assert_eq!(locale.country, Some("YU".to_string()));
    assert_eq!(locale.modifier, Some("Latn".to_string()));

    // All components
    let locale = Locale::from_string("en_US.UTF-8@euro");
    assert_eq!(locale.lang, "en");
    assert_eq!(locale.country, Some("US".to_string()));
    assert_eq!(locale.encoding, Some("UTF-8".to_string()));
    assert_eq!(locale.modifier, Some("euro".to_string()));
}

#[test]
fn test_locale_matching() {
    use xdg_desktop_entry::LocalizedString;

    let mut name = LocalizedString::new("Default");
    name.add_localized(Locale::from_string("en"), "English".to_string());
    name.add_localized(Locale::from_string("en_US"), "American English".to_string());
    name.add_localized(Locale::from_string("fr"), "Français".to_string());

    // Exact match
    assert_eq!(name.get(&Locale::from_string("en_US")), "American English");

    // Fall back to language only
    assert_eq!(name.get(&Locale::from_string("en_GB")), "English");

    // Fall back to default
    assert_eq!(name.get(&Locale::from_string("de")), "Default");
}

#[test]
fn test_serialization_roundtrip() {
    // Parse a file
    let original = DesktopEntry::parse_file("tests/fixtures/valid/minimal.desktop").unwrap();

    // Serialize it
    let serialized = original.serialize();

    // Parse it again
    let reparsed = DesktopEntry::parse(&serialized).unwrap();

    // Check that key fields match
    assert_eq!(reparsed.entry_type, original.entry_type);
    assert_eq!(reparsed.name.default, original.name.default);
    assert_eq!(reparsed.exec, original.exec);
}

#[test]
fn test_validation_link_without_url() {
    use xdg_desktop_entry::LocalizedString;

    let entry = DesktopEntry::new(DesktopEntryType::Link, LocalizedString::new("Test Link"));

    assert!(entry.validate().is_err());
}

#[test]
fn test_validation_link_with_url() {
    use xdg_desktop_entry::LocalizedString;

    let mut entry = DesktopEntry::new(DesktopEntryType::Link, LocalizedString::new("Test Link"));
    entry.url = Some("https://example.com".to_string());

    assert!(entry.validate().is_ok());
}

#[test]
fn test_validation_application_without_exec() {
    use xdg_desktop_entry::LocalizedString;

    let entry = DesktopEntry::new(
        DesktopEntryType::Application,
        LocalizedString::new("Test App"),
    );

    assert!(entry.validate().is_err());
}

#[test]
fn test_validation_application_with_exec() {
    use xdg_desktop_entry::LocalizedString;

    let mut entry = DesktopEntry::new(
        DesktopEntryType::Application,
        LocalizedString::new("Test App"),
    );
    entry.exec = Some("test-app".to_string());

    assert!(entry.validate().is_ok());
}

#[test]
fn test_validation_application_with_dbus() {
    use xdg_desktop_entry::LocalizedString;

    let mut entry = DesktopEntry::new(
        DesktopEntryType::Application,
        LocalizedString::new("Test App"),
    );
    entry.dbus_activatable = Some(true);

    assert!(entry.validate().is_ok());
}

// ============================================================================
// Additional invalid fixture tests
// ============================================================================

#[test]
fn test_invalid_type() {
    let result = DesktopEntry::parse_file("tests/fixtures/invalid/invalid_type.desktop");
    assert!(result.is_err());
    match result {
        Err(DesktopEntryError::InvalidValue(_, _)) => {}
        _ => panic!("Expected InvalidValue error"),
    }
}

#[test]
fn test_missing_type() {
    let result = DesktopEntry::parse_file("tests/fixtures/invalid/missing_type.desktop");
    assert!(result.is_err());
    match result {
        Err(DesktopEntryError::MissingRequiredKey(_)) => {}
        _ => panic!("Expected MissingRequiredKey error"),
    }
}

#[test]
fn test_missing_name() {
    let result = DesktopEntry::parse_file("tests/fixtures/invalid/missing_name.desktop");
    assert!(result.is_err());
    match result {
        Err(DesktopEntryError::MissingRequiredKey(_)) => {}
        _ => panic!("Expected MissingRequiredKey error"),
    }
}

#[test]
fn test_link_validation_failure() {
    let entry =
        DesktopEntry::parse_file("tests/fixtures/invalid/link_without_url.desktop").unwrap();
    assert!(entry.validate().is_err());
}

#[test]
fn test_app_validation_failure() {
    let entry =
        DesktopEntry::parse_file("tests/fixtures/invalid/app_without_exec.desktop").unwrap();
    assert!(entry.validate().is_err());
}

#[test]
fn test_invalid_group_header() {
    let result = DesktopEntry::parse_file("tests/fixtures/invalid/invalid_group_header.desktop");
    assert!(result.is_err());
    match result {
        Err(DesktopEntryError::InvalidGroupHeader(_, _)) => {}
        _ => panic!("Expected InvalidGroupHeader error"),
    }
}

#[test]
fn test_invalid_line_format() {
    let result = DesktopEntry::parse_file("tests/fixtures/invalid/invalid_line_format.desktop");
    assert!(result.is_err());
    match result {
        Err(DesktopEntryError::InvalidLine(_, _)) => {}
        _ => panic!("Expected InvalidLine error"),
    }
}

// ============================================================================
// Additional valid fixture tests
// ============================================================================

#[test]
fn test_parse_link_entry() {
    let entry = DesktopEntry::parse_file("tests/fixtures/valid/link_entry.desktop").unwrap();

    assert_eq!(entry.entry_type, DesktopEntryType::Link);
    assert_eq!(entry.name.default, "Example Website Link");
    assert_eq!(entry.url, Some("https://www.example.com".to_string()));
    assert!(entry.validate().is_ok());
}

#[test]
fn test_parse_directory_entry() {
    let entry = DesktopEntry::parse_file("tests/fixtures/valid/directory_entry.desktop").unwrap();

    assert_eq!(entry.entry_type, DesktopEntryType::Directory);
    assert_eq!(entry.name.default, "Custom Directory");
    assert!(entry.icon.is_some());
}

#[test]
fn test_parse_terminal_app() {
    let entry = DesktopEntry::parse_file("tests/fixtures/valid/terminal_app.desktop").unwrap();

    assert_eq!(entry.entry_type, DesktopEntryType::Application);
    assert_eq!(entry.terminal, Some(true));
    assert!(entry.validate().is_ok());
}

#[test]
fn test_parse_dbus_app() {
    let entry = DesktopEntry::parse_file("tests/fixtures/valid/dbus_app.desktop").unwrap();

    assert_eq!(entry.entry_type, DesktopEntryType::Application);
    assert_eq!(entry.dbus_activatable, Some(true));
    assert!(entry.validate().is_ok());
}

#[test]
fn test_parse_hidden_app() {
    let entry = DesktopEntry::parse_file("tests/fixtures/valid/hidden_app.desktop").unwrap();

    assert_eq!(entry.hidden, Some(true));
    assert_eq!(entry.no_display, Some(true));
}

#[test]
fn test_parse_feature_rich() {
    let entry = DesktopEntry::parse_file("tests/fixtures/valid/feature_rich.desktop").unwrap();

    assert_eq!(entry.version, Some("1.5".to_string()));
    assert_eq!(entry.startup_notify, Some(true));

    // Check multiple localizations
    assert!(
        entry
            .name
            .localized
            .contains_key(&Locale::from_string("de"))
    );
    assert!(
        entry
            .name
            .localized
            .contains_key(&Locale::from_string("ja"))
    );

    // Check keywords
    assert!(entry.keywords.is_some());
    let keywords = entry.keywords.as_ref().unwrap();
    assert!(keywords.default.contains(&"feature".to_string()));

    // Check OnlyShowIn
    assert!(entry.only_show_in.is_some());

    // Check actions
    assert!(entry.actions.is_some());
    assert!(entry.additional_groups.contains_key("Desktop Action edit"));
    assert!(entry.additional_groups.contains_key("Desktop Action view"));

    assert!(entry.validate().is_ok());
}
