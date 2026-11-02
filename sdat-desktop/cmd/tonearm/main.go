package main

import (
	"log/slog"
	"os"
	"strings"

	_ "codeberg.org/dergs/tonearm/internal/log"

	_ "codeberg.org/dergs/tonearm/internal/icons"
	_ "codeberg.org/dergs/tonearm/internal/styles"

	_ "codeberg.org/dergs/tonearm/internal/features/scrobbling"
	_ "codeberg.org/dergs/tonearm/internal/services"

	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/secrets"
	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/internal/ui"
	"codeberg.org/dergs/tonearm/pkg/schwifty/tracking"
	"codeberg.org/dergs/tonearm/pkg/utils/imgutil"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"github.com/infinytum/injector"
)

func init() {
	slog.SetLogLoggerLevel(slog.LevelInfo)
	if os.Getenv("TONEARM_DEBUG") == "1" {
		go tracking.LogAliveObjects()
	}
}

var app *adw.Application

func main() {
	app = adw.NewApplication("dev.dergs.Tonearm", gio.GApplicationHandlesCommandLineValue)
	defer app.Unref()
	app.ConnectActivate(new(onActivate))
	app.ConnectCommandLine(new(onCommandLine))
	injector.Singleton(func() *adw.Application {
		return app
	})

	injector.Singleton(func() *imgutil.ImgUtil {
		return imgutil.NewImgUtil()
	})

	if code := app.Run(int32(len(os.Args)), os.Args); code > 0 {
		app.Quit()
		os.Exit(int(code))
	}
}

func onActivate(_ gio.Application) {
	window := ui.NewWindow(app)
	window.Present()
	window.ConnectCloseRequest(new(func(w gtk.Window) bool {
		// Only allow running in background if there is a track playing,
		// so that the user can bring the app back up with MPRIS
		if settings.General().ShouldRunInBackground() && player.TrackChanged.CurrentValue() != nil {
			window.Hide()
			return true
		}
		return false
	}))

	if err := secrets.Healthcheck(); err != nil {
		slog.Error("Secret service health check failed", "title", err.Title, "body", err.Body, "fatal", err.Fatal)
		window.PresentSecretServiceError(err)
	}

}

var isActive bool

func onCommandLine(app gio.Application, ptr uintptr) int32 {
	cmd := gio.ApplicationCommandLineNewFromInternalPtr(ptr)
	args := cmd.GetArguments(nil)
	if !isActive {
		app.Activate()
		isActive = true
	} else {
		window := injector.MustInject[*adw.ApplicationWindow]()
		window.Show()
	}

	if len(args) == 2 {
		url := args[1]
		if after, ok := strings.CutPrefix(url, "tidal://track/"); ok {
			player.PlayTrackID(after)
		} else {
			router.Navigate(url)
		}
	}
	return 0
}
