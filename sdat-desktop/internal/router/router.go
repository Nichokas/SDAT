package router

import (
	"log/slog"
	"strings"
	"time"
)

var logger = slog.With("module", "router")

func Navigate(path string) {
	navigate(strings.TrimPrefix(path, "tidal://"), false)
}

func navigate(path string, offRecord bool) {
	if history.IsCurrentlyOn(path) && !offRecord {
		logger.Debug("skipped navigation as we are already on the same page")
		return
	}

	logger.Debug("navigation started")
	// We are starting to navigate, notify the rest of the application
	Navigation.Notify(&NavigationEvent{
		Completed: false,
		Path:      path,
	})

	// If the route is not registered, use the not found handler
	handler := findHandler(path)
	if handler == nil {
		logger.Info("no handler found", "path", path)
		handler = notFoundHandler
	}

	startTime := time.Now()
	logger.Debug("executing route handler", "path", path, "started_at", startTime)
	go func(path string, handler Handler) {
		response, shouldCache := executeHandler(handler)
		logger.Info("navigation completed", "path", path, "duration_ms", time.Since(startTime).Milliseconds(), "should_cache", shouldCache)
		entry := &HistoryEntry{Path: path, PageTitle: response.PageTitle, ExpiresAt: response.ExpiresAt}
		if response.Toolbar != nil {
			entry.Toolbar = response.Toolbar.ToGTK()
		}
		if response.View != nil {
			entry.View = response.View.ToGTK()
		}
		if !shouldCache {
			entry.ExpiresAt = new(time.Now())
		}
		if !offRecord {
			history.Push(entry)
		}
		handleNavigationComplete(entry)

	}(path, handler)
}

func Back() {
	if len(history.Entries) == 0 {
		return
	}

	previous := history.Pop()
	if previous == nil {
		return
	}

	if previous.View != nil {
		handleNavigationComplete(previous)
	} else {
		navigate(previous.Path, true)
	}
}

func Refresh() {
	if history.Current == nil {
		return
	}

	history.Current.Toolbar = nil
	history.Current.View = nil

	navigate(history.Current.Path, true)
}

func Current() *HistoryEntry {
	return history.Current
}

func Clear() {
	history.Clear()
}
