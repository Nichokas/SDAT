package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen SearchEntry *gtk.SearchEntry gtk

func (f SearchEntry) ConnectActivate(cb func(gtk.SearchEntry)) SearchEntry {
	return func() *gtk.SearchEntry {
		searchEntry := f()
		callback.HandleCallback(searchEntry.Object, "activate", cb)
		return searchEntry
	}
}

func (f SearchEntry) ConnectSearchChanged(cb func(gtk.SearchEntry)) SearchEntry {
	return func() *gtk.SearchEntry {
		searchEntry := f()
		callback.HandleCallback(searchEntry.Object, "search-changed", cb)
		return searchEntry
	}
}

func (f SearchEntry) PlaceholderText(text string) SearchEntry {
	return func() *gtk.SearchEntry {
		searchEntry := f()
		searchEntry.SetPlaceholderText(text)
		return searchEntry
	}
}

func (f SearchEntry) SearchDelay(delay uint32) SearchEntry {
	return func() *gtk.SearchEntry {
		searchEntry := f()
		searchEntry.SetSearchDelay(delay)
		return searchEntry
	}
}
