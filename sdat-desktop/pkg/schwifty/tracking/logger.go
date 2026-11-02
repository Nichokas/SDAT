package tracking

import (
	"log/slog"
	"time"
)

var logger = slog.With("library", "schwifty", "module", "tracking")

func LogAliveObjects() {
	lastCount := 0
	for {
		objects := Alive()
		if lastCount != len(objects) {
			lastCount = len(objects)
			slog.Info("alive object count", "count", len(objects))
		}
		time.Sleep(1 * time.Second)
	}
}
