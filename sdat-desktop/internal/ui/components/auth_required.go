package components

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func AuthRequired(description string) schwifty.StatusPage {
	return StatusPage().
		IconName("avatar-default-symbolic").
		Title(gettext.Get("Authentication Required")).
		Description(description).
		Child(
			Button().
				Label(gettext.Get("Sign In…")).
				WithCSSClass("pill").
				WithCSSClass("suggested-action").
				ActionName("win.sign-in").
				HAlign(gtk.AlignCenterValue),
		)
}
