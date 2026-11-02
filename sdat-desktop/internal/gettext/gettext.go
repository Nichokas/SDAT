package gettext

import (
	"embed"
	"log/slog"

	golocale "github.com/jeandeaual/go-locale"
	"github.com/leonelquinteros/gotext"
)

//go:embed locales
var locales embed.FS

//go:generate go run codeberg.org/dergs/tonearm/internal/gettext/gen locales/tonearm.pot
var locale *gotext.Locale

func init() {
	userLocale, err := golocale.GetLocale()
	if err != nil {
		slog.Error("could not detect system language, falling back to english")
		userLocale = "en_US"
	}
	locale = gotext.NewLocaleFSWithPath(userLocale, locales, "locales")
	locale.AddDomain("default")
}

func Get(msgid string, args ...any) string {

	return locale.Get(msgid, args...)
}

func GetN(msgid string, msgidPlural string, n int, args ...any) string {
	return locale.GetN(msgid, msgidPlural, n, args...)
}
