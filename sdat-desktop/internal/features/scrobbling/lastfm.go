package scrobbling

import (
	"context"
	"strings"
	"time"

	"codeberg.org/dergs/tonearm/internal/features/scrobbling/lastfm"
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/tonearm"

	adwbindings "codeberg.org/dergs/tonearm/pkg/schwifty/bindings/adw"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"github.com/infinytum/injector"

	lastfmlib "github.com/twoscott/gobble-fm/lastfm"
)

type LastFm struct {
	Client *lastfm.Client
}

var LastFmScrobbler = LastFm{
	Client: lastfm.NewClient(),
}

func init() {
	Scrobblers = append(Scrobblers, &LastFmScrobbler)
}

func loadingDialog(pollingCancel context.CancelFunc, browserUri string) adwbindings.AlertDialog {
	window := injector.MustInject[*gtk.Window]()
	var self weak.WidgetRef

	return AlertDialog(
		gettext.Get("Logging in to Last.fm"),
		gettext.Get("Continue the authentication process in your browser."),
	).
		ConnectConstruct(func(ad *adw.AlertDialog) {
			self = weak.NewWidgetRef(ad)
		}).
		WithCSSClass("no-response").
		ConnectClosed(func(d adw.Dialog) {
			pollingCancel()
		}).
		ExtraChild(
			VStack(
				Spinner().SizeRequest(128, 128),
				Button().
					Label(gettext.Get("Reopen Browser")).
					ConnectClicked(func(b gtk.Button) {
						gtk.ShowUri(window, browserUri, uint32(time.Now().Unix()))
					}).
					CornerRadius(12).
					VPadding(10).
					HPadding(20).
					MinHeight(24).
					MinWidth(60),
				Button().
					Label(gettext.Get("Cancel")).
					ConnectClicked(func(b gtk.Button) {
						self.Use(func(obj *gtk.Widget) {
							adw.AlertDialogNewFromInternalPtr(obj.Ptr).Close()
						})
					}).
					CornerRadius(12).
					VPadding(10).
					HPadding(20).
					MinHeight(24).
					MinWidth(60),
			).
				Spacing(12),
		)
}

func (scrobbler *LastFm) Configure() (completed bool, err error) {
	window, _ := injector.Inject[*gtk.Window]()

	token, err := scrobbler.Client.Auth.Token()
	if err != nil {
		return false, err
	}

	uri := scrobbler.Client.AuthTokenURL(token)

	pollingCtx, pollingCancel := context.WithCancel(context.Background())
	dialog := loadingDialog(pollingCancel, uri)()
	ref := weak.NewWidgetRef(dialog)

	dialog.Ref()
	schwifty.OnMainThreadOncePure(func() {
		defer dialog.Unref()
		dialog.Present(&window.Widget)
	})

	defer ref.Use(func(obj *gtk.Widget) {
		adw.AlertDialogNewFromInternalPtr(obj.Ptr).Close()
	})

	gtk.ShowUri(window, uri, uint32(time.Now().Unix()))

	ok, err := scrobbler.Client.StartPollingSession(pollingCtx, token)
	if err != nil || !ok {
		return ok, err
	}

	settings.Scrobbling().SetLastFMToken(scrobbler.Client.SessionKey)
	user, err := scrobbler.Client.User.SelfInfo()
	if err != nil {
		return false, err
	}

	logger.Info("Last.fm configured", "username", user.Name)

	return true, nil
}

func (scrobbler *LastFm) Unconfigure() error {
	settings.Scrobbling().SetLastFMToken("")
	scrobbler.Client.SetSessionKey("")
	return nil
}

func (scrobbler *LastFm) IsConfigured() bool {
	if !settings.Scrobbling().ShouldEnableLastFM() {
		logger.Debug("skipping Last.fm because it is disabled")
		return false
	}

	if scrobbler.Client.SessionKey == "" {
		logger.Debug("skipping Last.fm because it doesn't have a session key")
		return false
	}
	user, err := scrobbler.Client.User.SelfInfo()
	if err != nil {
		logger.Error("error while fetching user", "error", err)
		return false
	}

	return user != nil
}

func (scrobbler *LastFm) GetName() string {
	return "Last.fm"
}

func (scrobbler *LastFm) NowPlaying(track tonearm.Track) {
	if !scrobbler.IsConfigured() {
		return
	}

	_, err := scrobbler.Client.Track.UpdateNowPlaying(lastfmlib.UpdateNowPlayingParams{
		Artist:   strings.Join(track.Artists().Names(), ", "),
		Track:    track.Title(),
		Album:    track.Album().Title(),
		Duration: lastfmlib.Duration(track.Duration()),
	})

	if err == nil {
		return
	} else {
		logger.Error("error while updating now playing", "error", err)
		return
	}
}
func (scrobbler *LastFm) Scrobble(event *ScrobbleEvent) {
	if !scrobbler.IsConfigured() {
		return
	}

	_, err := scrobbler.Client.Track.Scrobble(lastfmlib.ScrobbleParams{
		Artist:   strings.Join(event.Track.Artists().Names(), ", "),
		Track:    event.Track.Title(),
		Album:    event.Track.Album().Title(),
		Duration: lastfmlib.Duration(event.Track.Duration()),

		Time: time.Now(),
	})

	if err == nil {
		return
	} else {
		logger.Error("error while scrobbling", "error", err)
		return
	}
}
