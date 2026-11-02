package ui

import (
	"fmt"
	"regexp"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

var (
	Commit, Version string
)

func isStable() bool {
	if Version != "" {
		return true
	}

	if Commit == "" {
		// If no commit is available.
		// This happens if the app is run with `go run ./cmd/tonearm`.

		return false
	} else if ok, _ := regexp.MatchString(`^.*-\d+-g[0-9a-f]{7}$`, Commit); ok {
		// If the commit matches the git describe scheme.
		// This happens when the code is run locally, through a project-provided tool, and a tag is available.

		regex := regexp.MustCompile(`^(.*)-(\d+)-g([0-9a-f]{7})`)
		parts := regex.FindStringSubmatch(Commit)

		offset := parts[2]

		if offset != "0" {
			return false
		}

		return true
	} else {
		// If the commit is not a git describe scheme.
		// This happens when the code is run locally, through a project-provided tool, and no tag is available.
		return false
	}
}

func getVersionPrefix(version string) (prefix string) {
	if version[0] == 'v' {
		if version[1] == '0' {
			prefix = "α "
		}
	} else {
		if version[0] == '0' {
			prefix = "α "
		} else {
			prefix = "v"
		}
	}

	return
}

func getVersionNumber() string {
	if Version == "" {
		// If no version is available.
		// This happens if the go binary is run directly, or through the Zed task

		if Commit == "" {
			// If no commit is available.
			// This happens if the app is run with `go run ./cmd/tonearm`.

			return "local"
		} else if ok, _ := regexp.MatchString(`^.*-\d+-g[0-9a-f]{7}$`, Commit); ok {
			// If the commit matches the git describe scheme.
			// This happens when the code is run locally, through a project-provided tool, and a tag is available.
			// We can therefore display a git version, in the `v1.0.0 β(r1.0123abc)` scheme.

			regex := regexp.MustCompile(`^(.*)-(\d+)-g([0-9a-f]{7})`)
			parts := regex.FindStringSubmatch(Commit)

			_, tag, offset, commitHash := parts[0], parts[1], parts[2], parts[3]
			if tag[0] == 'v' {
				tag = tag[1:]
			}

			prefix := getVersionPrefix(tag)

			suffix := ""
			if offset != "0" {
				suffix = "β"
			}

			return fmt.Sprintf("%s%s %s(r%s.%s)", prefix, tag, suffix, offset, commitHash)
		} else {
			// If the commit is not a git describe scheme.
			// This happens when the code is run locally, through a project-provided tool, and no tag is available.
			// We can therefore display a commit hash, with the alpha symbol.
			return fmt.Sprintf("α git+%s", Commit[:7])
		}
	} else {
		// If a version has been provided.
		// We can therefore display the version, and the commit hash for clarity.

		commit := Commit
		suffix := ""
		if commit != "" {
			suffix = fmt.Sprintf(" (%s)", commit[:7])
		}

		version := Version
		if version[0] == 'v' {
			version = version[1:]
		}

		prefix := getVersionPrefix(version)

		return fmt.Sprintf("%s%s%s", prefix, version, suffix)
	}
}

func (w *Window) PresentAbout() {

	about := adw.NewAboutDialog()
	about.SetApplicationIcon("logo")
	about.SetApplicationName("Tonearm")
	about.SetVersion(getVersionNumber())
	about.SetLicenseType(gtk.LicenseGpl30Value)
	about.SetDevelopers([]string{
		"Nila The Dragon https://github.com/NilaTheDragon",
		"Dråfølin https://github.com/Drafolin",
	})
	about.SetTranslatorCredits(gettext.Get("translator-credits"))
	about.SetCopyright("© 2026 Nila The Dragon")
	about.SetWebsite("https://dergs.dev/projects/tonearm")
	about.SetIssueUrl("https://codeberg.org/dergs/Tonearm/issues")
	about.SetSupportUrl("https://matrix.to/#/%23tonearm:derg.social")

	about.AddLegalSection("GStreamer Bindings (go-gst/go-gst)", "© 2020 https://github.com/go-gst/go-gst", gtk.LicenseLgpl30Value, "")
	about.AddLegalSection("DBus Client (godbus/dbus)", "© 2020 Georg Reinke https://github.com/godbus/dbus", gtk.LicenseBsdValue, "")
	about.AddLegalSection("UUID (google/uuid)", "© 2009, 2014 Google Inc. https://github.com/google/uuid", gtk.LicenseBsd3Value, "")
	about.AddLegalSection("Dependency Injector (infinytum/injector)", "© 2022 Infinytum https://github.com/infinytum/injector", gtk.LicenseUnknownValue, "")
	about.AddLegalSection("System Locale Detection (jeandeaual/go-locale)", "© 2020 Alexis Jeandeau https://github.com/jeandeaual/go-locale", gtk.LicenseMitX11Value, "")
	about.AddLegalSection("Go Keychain (keybase/go-keychain)", "© 2015 Keybase https://github.com/keybase/go-keychain", gtk.LicenseMitX11Value, "")
	about.AddLegalSection("GTK4 / Libadwaita Bindings (jwijenbergh/puregotk)", "© 2022 Kyle McGough https://codeberg.org/puregotk/puregotk", gtk.LicenseMitX11Value, "")
	about.AddLegalSection("JSON Merger (qjebbs/go-jsons)", "© 2022 Jebbs https://github.com/qjebbs/go-jsons", gtk.LicenseMitX11Value, "")
	about.AddLegalSection("ISO8601 Duration Parser (osodev/duration)", "© 2023 Jeroen Wijenbergh https://github.com/sosodev/duration", gtk.LicenseMitX11Value, "")
	about.AddLegalSection("QR Code Generator (yeqown/go-qrcode)", "© 2018 yeqown https://github.com/yeqown/go-qrcode", gtk.LicenseMitX11Value, "")
	about.AddLegalSection("Gettext (leonelquinteros/gotext)", "© 2016 Leonel Quinteros https://github.com/leonelquinteros/gotext", gtk.LicenseMitX11Value, "")
	about.AddLegalSection("Gobble.fm (twoscott/gobble-fm)", "© 2025 twoscott https://github.com/twoscott/gobble-fm", gtk.LicenseMitX11Value, "")
	about.AddLegalSection("slogpretty (Marlliton/slogpretty)", "© 2025 Marlliton Souza https://github.com/Marlliton/slogpretty", gtk.LicenseMitX11Value, "")
	about.AddLegalSection("ttlcache (jellydator/ttlcache)", "© 2022 Jellydator https://github.com/jellydator/ttlcache", gtk.LicenseMitX11Value, "")
	about.Present(&w.Widget)
	about.Unref()
}
