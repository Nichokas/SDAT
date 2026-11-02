//go:build linux

package player

import (
	"fmt"
	"math"
	"os"
	"time"

	"codeberg.org/dergs/tonearm/internal/g"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/mpris"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"github.com/godbus/dbus/v5"
	"github.com/infinytum/injector"
)

var mprisServer = g.Lazy(func() *mpris.Server {
	mprisServer := mpris.NewMprisServer("org.mpris.MediaPlayer2.dev.dergs.Tonearm", "dev.dergs.Tonearm", "Tonearm")
	mprisServer.OnPause(Pause)
	mprisServer.OnPlayPause(PlayPause)
	mprisServer.OnPlay(Play)
	mprisServer.OnTrackNext(Next)
	mprisServer.OnTrackPrevious(Previous)
	mprisServer.OnQuit(func() {
		app := injector.MustInject[*adw.ApplicationWindow]()
		app.GetApplication().Quit()
		os.Exit(0)
	})
	mprisServer.OnRaise(func() {
		window := injector.MustInject[*adw.ApplicationWindow]()
		window.Show()
		window.Present()
	})
	mprisServer.OnLoopStatusChanged(func(loopStatus mpris.LoopStatus) {
		switch loopStatus {
		case mpris.LoopNone:
			go SetRepeatMode(RepeatModeNone)
		case mpris.LoopTrack:
			go SetRepeatMode(RepeatModeTrack)
		case mpris.LoopPlaylist:
			go SetRepeatMode(RepeatModeQueue)
		}
	})
	mprisServer.OnSeek(func(offset time.Duration) {
		SeekToPosition(PlaybackStateChanged.CurrentValue().Position+offset, false)
	})
	mprisServer.OnSetPosition(func(position time.Duration) { SeekToPosition(position, false) })
	mprisServer.OnVolumeChanged(func(newVal float64) {
		SetVolume(newVal)
	})
	mprisServer.OnShuffleChanged(func(shuffle bool) {
		currentTrack := ""
		if current := TrackChanged.CurrentValue(); current != nil {
			currentTrack = current.ID()
		}
		SetShuffle(shuffle, currentTrack)
	})
	mprisServer.Export()
	return mprisServer
})

func init() {
	PlaybackStateChanged.Signal.On(func(state PlaybackState) bool {
		mprisServer().SetPosition(state.Position, false)
		if state.Loading {
			mprisServer().SetPlaybackStatus(mpris.PlaybackStatusPaused)
		} else {
			switch state.Status {
			case PlaybackStatusPaused:
				mprisServer().SetPlaybackStatus(mpris.PlaybackStatusPaused)
			case PlaybackStatusPlaying:
				mprisServer().SetPlaybackStatus(mpris.PlaybackStatusPlaying)
			case PlaybackStatusStopped:
				mprisServer().SetPlaybackStatus(mpris.PlaybackStatusStopped)
			}
		}

		return signals.Continue
	})

	RepeatModeChanged.On(func(rm RepeatMode) bool {
		switch rm {
		case RepeatModeNone:
			mprisServer().SetLoopStatus(mpris.LoopNone)
		case RepeatModeTrack:
			mprisServer().SetLoopStatus(mpris.LoopTrack)
		case RepeatModeQueue:
			mprisServer().SetLoopStatus(mpris.LoopPlaylist)
		}
		return signals.Continue
	})

	TrackChanged.On(func(trackInfo *TrackChangedEvent) bool {
		if trackInfo == nil {
			mprisServer().SetTrackMetadata(map[string]any{})
			mprisServer().Disconnect()

			return signals.Continue
		}

		mprisServer().Connect()

		artistNames := []string{}
		for _, artist := range trackInfo.Artists() {
			artistNames = append(artistNames, artist.Title())
		}

		mprisServer().SetTrackMetadata(map[string]any{
			"mpris:trackid": dbus.ObjectPath("/org/mpris/MediaPlayer2/TrackList/Track" + trackInfo.ID()),
			"mpris:artUrl":  trackInfo.Cover(math.MaxInt).Static(),
			"mpris:length":  trackInfo.Duration().Microseconds(),
			"xesam:album":   trackInfo.Album().Title(),
			"xesam:artist":  artistNames,
			"xesam:title":   trackInfo.Title(),
			"xesam:url":     fmt.Sprintf("https://tidal.com/track/%s", trackInfo.ID()),
		})

		return signals.Continue
	})

	VolumeChanged.On(func(volume float64) bool {
		go mprisServer().SetVolume(volume)
		return signals.Continue
	})

	Seeked.On(func(position time.Duration) bool {
		go mprisServer().SetPosition(position, true)
		return signals.Continue
	})
}
