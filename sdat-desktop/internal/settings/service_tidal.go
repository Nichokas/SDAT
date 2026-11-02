package settings

import (
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/gobject"
)

type ServiceTidalSettings struct {
	settings *gio.Settings
}

func (s *ServiceTidalSettings) APIBaseURL() string {
	return s.settings.GetString("api-baseurl")
}

func (s *ServiceTidalSettings) BindAPIBaseURL(target *gobject.Object, property string) {
	s.settings.Bind("api-baseurl", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (s *ServiceTidalSettings) OpenAPIBaseURL() string {
	return s.settings.GetString("openapi-baseurl")
}

func (s *ServiceTidalSettings) BindOpenAPIBaseURL(target *gobject.Object, property string) {
	s.settings.Bind("openapi-baseurl", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (s *ServiceTidalSettings) ResourcesBaseURL() string {
	return s.settings.GetString("resources-baseurl")
}

func (s *ServiceTidalSettings) BindResourcesBaseURL(target *gobject.Object, property string) {
	s.settings.Bind("resources-baseurl", target, property, gio.GSettingsBindNoSensitivityValue)
}

func (s *ServiceTidalSettings) AuthBaseURL() string {
	return s.settings.GetString("auth-baseurl")
}

func (s *ServiceTidalSettings) BindAuthBaseURL(target *gobject.Object, property string) {
	s.settings.Bind("auth-baseurl", target, property, gio.GSettingsBindNoSensitivityValue)
}
