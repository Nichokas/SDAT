package scrobbling

import (
	"log/slog"
	"time"

	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var logger = slog.With("module", "scrobbler")

var Scrobblers []Scrobbler

type Scrobbler interface {
	NowPlaying(tonearm.Track)
	Scrobble(*ScrobbleEvent)
	IsConfigured() bool
	GetName() string
}

type ScrobbleEvent struct {
	Track      tonearm.Track
	ListenedAt time.Time
}

func init() {
	var scrobbleClock *Clock
	player.TrackChanged.On(func(t *player.TrackChangedEvent) bool {
		if t == nil {
			return signals.Continue
		}

		logger.Debug("notifying scrobblers that a new track has started playing")
		for _, scrobbler := range Scrobblers {
			if !scrobbler.IsConfigured() {
				logger.Debug("skipping now playing event", "service", scrobbler.GetName())
				continue
			}

			logger.Debug("sending NowPlaying event", "service", scrobbler.GetName())
			go scrobbler.NowPlaying(t)
		}

		if scrobbleClock != nil {
			scrobbleClock.Stop()
		}
		scrobbleClock = newClock(t)
		return signals.Continue
	})

	player.PlaybackStateChanged.On(func(ps player.PlaybackState) bool {
		if scrobbleClock == nil {
			return signals.Continue
		}

		if ps.Status == player.PlaybackStatusPlaying {
			scrobbleClock.Start()
		} else {
			scrobbleClock.Stop()
		}
		return signals.Continue
	})
}
