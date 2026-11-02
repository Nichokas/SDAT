package log

import (
	"log/slog"
	"os"

	"github.com/Marlliton/slogpretty"
)

func init() {
	if os.Getenv("TONEARM_DEBUG") == "1" {
		handler := slogpretty.New(os.Stdout, &slogpretty.Options{
			Level:      slog.LevelDebug,
			Colorful:   true,                         // Enable colors. Default is true
			AddSource:  true,                         // Show file location
			Multiline:  false,                        // Pretty print for complex data
			TimeFormat: slogpretty.DefaultTimeFormat, // Custom format (e.g., time.Kitchen)
		})
		slog.SetDefault(slog.New(handler))
	}
}
