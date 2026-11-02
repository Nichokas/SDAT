package scrobbling

import (
	"context"
	"sync/atomic"
	"time"

	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

type Clock struct {
	counter   int
	track     tonearm.Track
	isRunning bool
	scrobbled atomic.Bool
	startedAt time.Time

	context    context.Context
	cancelFunc context.CancelFunc
}

func (c *Clock) Start() {
	if c.isRunning || c.scrobbled.Load() {
		return
	}
	c.isRunning = true

	ctx, cancel := context.WithCancel(context.Background())
	c.context = ctx
	c.cancelFunc = cancel

	go func() {
		defer func() {
			c.isRunning = false
		}()

		for {
			select {
			case <-c.context.Done():
				return
			case <-time.After(time.Second):
				c.counter++
				if c.counter >= int(c.track.Duration().Seconds())/2 || c.counter >= int((time.Minute*4).Seconds()) {
					if c.scrobbled.Swap(true) {
						return
					}

					logger.Debug("notifying scrobblers that a track should be scrobbled")

					event := new(ScrobbleEvent)
					event.Track = c.track
					event.ListenedAt = c.startedAt

					for _, scrobbler := range Scrobblers {
						if !scrobbler.IsConfigured() {
							logger.Debug("skipping scrobbling", "service", scrobbler.GetName())
							continue
						}

						logger.Debug("sending Scrobble event", "service", scrobbler.GetName())
						go scrobbler.Scrobble(event)
					}

					c.cancelFunc()
					return
				}
			}
		}
	}()
}

func (c *Clock) Stop() {
	if !c.isRunning {
		return
	}
	c.cancelFunc()
}

func newClock(track tonearm.Track) *Clock {
	return &Clock{
		track:     track,
		startedAt: time.Now(),
	}
}
