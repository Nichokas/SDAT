package search

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
)

func PromptView() schwifty.StatusPage {
	return StatusPage().
		IconName("loupe-symbolic").
		Title(gettext.Get("Search")).
		Description(gettext.Get("Start typing in the search bar to search for songs, artists, albums or playlists"))
}

func LoadingView() schwifty.Clamp {
	return Clamp().
		MaximumSize(50).
		Child(Spinner())
}
