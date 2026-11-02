package router

import (
	"errors"
	"fmt"
	"runtime/debug"
)

type Handler func() *Response

func executeHandler(handler Handler) (response *Response, shouldCache bool) {
	// In case the handler fatally fails, we want to show a generic error page
	defer func() {
		if err := recover(); err != nil {
			logger.Error("handler panicked", "error", err)
			fmt.Println(string(debug.Stack()))
			response = errorHandler(err.(error))
			shouldCache = false
		}
	}()

	response = handler()

	// If the handler didn't crash but provided no response, we assume this is an error
	if response == nil {
		logger.Error("handler returned no result")
		response = errorHandler(errors.New("route handler did not generate any response"))
		shouldCache = false
		return
	}

	// If the handler returned an error, we generate an error page for it
	if response.Error != nil {
		logger.Error("handler failed", "error", response.Error)
		fmt.Println(string(debug.Stack()))
		response = errorHandler(response.Error)
		shouldCache = false
		return
	}

	shouldCache = true
	return
}

func handleNavigationComplete(entry *HistoryEntry) {
	Navigation.Notify(&NavigationEvent{
		Completed: true,
		Path:      entry.Path,
		Result:    entry,
	})
}
