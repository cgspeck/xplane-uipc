## ADDED Requirements

### Requirement: Shared mapping loader
The system SHALL provide a single shared function to load and resolve mappings from the plugin directory's `mappings.toml` file, used both at startup and when the user triggers a reload.

#### Scenario: Startup loads mappings via shared loader
- **WHEN** the plugin starts up (during `XPluginEnable`)
- **THEN** the system SHALL call the shared mapping loader to load and resolve mappings from `mappings.toml` in the plugin directory
- **THEN** all resolved mappings SHALL be available in the plugin state

#### Scenario: Reload uses same shared loader
- **WHEN** the user clicks "Reload Mappings" in the plugin menu
- **THEN** the system SHALL call the same shared mapping loader used at startup, NOT a separate code path

#### Scenario: Path is absolute and based on X-Plane system path
- **WHEN** the shared mapping loader resolves the path to `mappings.toml`
- **THEN** the path SHALL be constructed from `XPLMGetSystemPath` + `Resources/plugins/xplane-uipc/mappings.toml`
- **THEN** the path SHALL be an absolute filesystem path

#### Scenario: Invalid file on reload preserves old mappings
- **WHEN** the user clicks "Reload Mappings" and the mappings file is missing or contains invalid TOML
- **THEN** the system SHALL log the error
- **THEN** the existing mappings SHALL remain unchanged and the plugin SHALL continue running normally

#### Scenario: Menu item exists
- **WHEN** the plugin is loaded
- **THEN** there SHALL be a "Reload Mappings" menu item under the X-Plane UIPC plugin menu
