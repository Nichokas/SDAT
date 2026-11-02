package router

import (
	"sync"
	"time"

	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

var history = &History{
	Mutex:   sync.Mutex{},
	Current: nil,
	Entries: []*HistoryEntry{},
}

type HistoryEntry struct {
	ExpiresAt *time.Time
	PageTitle string
	Path      string
	View      *gtk.Widget
	Toolbar   *gtk.Widget
}

type History struct {
	sync.Mutex
	Current *HistoryEntry
	Entries []*HistoryEntry
}

func (h *History) IsCurrentlyOn(path string) bool {
	if h.Current == nil {
		return false
	}

	if h.Current.Path != path {
		return false
	}

	return true
}

func (h *History) Pop() *HistoryEntry {
	h.Lock()
	defer h.Unlock()
	defer HistoryUpdated.Notify(h)

	if len(h.Entries) == 0 {
		return nil
	}

	h.Current = h.Entries[len(h.Entries)-1]
	h.Entries = h.Entries[:len(h.Entries)-1]

	if h.Current.ExpiresAt != nil && h.Current.ExpiresAt.Before(time.Now()) {
		h.Current.Toolbar = nil
		h.Current.View = nil
	}

	return h.Current
}

func (h *History) Push(entry *HistoryEntry) {
	h.Lock()
	defer h.Unlock()
	defer HistoryUpdated.Notify(h)

	if h.Current != nil {
		if int32(len(h.Entries)) >= settings.Performance().MaxRouterHistorySize() {
			h.Entries = h.Entries[1:]
		}

		h.Entries = append(h.Entries, h.Current)
	}

	h.Current = entry
}

func (h *History) Clear() {
	h.Lock()
	defer h.Unlock()
	defer HistoryUpdated.Notify(h)

	h.Current = nil
	h.Entries = []*HistoryEntry{}
}
