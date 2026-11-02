package router

import "codeberg.org/dergs/tonearm/internal/signals"

// HistoryUpdated is a signal that is emitted when the router updates the history.
// The history parameter is the new history that the router is updating.
// The history parameter cannot be nil.
var HistoryUpdated = signals.NewStatelessSignal[*History]()

type NavigationEvent struct {
	Completed bool
	Path      string
	Result    *HistoryEntry
}

// Navigation is a signal that is emitted when the router starts or completes a navigation.
// If Completed is true, the Result parameter is always set.
var Navigation = signals.NewStatelessSignal[*NavigationEvent]()
