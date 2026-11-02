package player

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

const (
	repeatListIconName  = "playlist-repeat-symbolic"
	repeatTrackIconName = "playlist-repeat-song-symbolic"
	pauseIconName       = "pause-symbolic"
	playIconName        = "play-symbolic"
)

var controlButton = Button().
	BindSensitive(isControllableState).
	MinHeight(34).MinWidth(34).
	WithCSSClass("flat").
	VAlign(gtk.AlignCenterValue)

func controls() schwifty.Box {
	var (
		pauseIcon   = Image().FromIconName(pauseIconName)()
		playIcon    = Image().FromIconName(playIconName)()
		playSpinner = Spinner().SizeRequest(16, 16)()
	)

	var (
		playPauseChildState       = state.NewStateful[any](nil)
		actualPlayPauseChildState = state.NewStateful[any](nil)
		playPauseTooltipState     = state.NewStateful("")
		playPauseSensitiveState   = state.NewStateful(false)
	)

	player.PlaybackStateChanged.On(func(state player.PlaybackState) bool {
		schwifty.OnMainThreadOncePure(func() {
			var val any
			switch state.Status {
			case player.PlaybackStatusPlaying:
				val = pauseIcon
				playPauseTooltipState.SetValue(gettext.Get("Pause"))
			case player.PlaybackStatusPaused, player.PlaybackStatusStopped:
				val = playIcon
				playPauseTooltipState.SetValue(gettext.Get("Play"))
			}

			if isControllableState.Value() {
				playPauseSensitiveState.SetValue(isTrackLoadedState.Value())
				actualPlayPauseChildState.SetValue(val)
			} else {
				playPauseSensitiveState.SetValue(false)
				actualPlayPauseChildState.SetValue(playSpinner)
			}

			playPauseChildState.SetValue(val)
		})

		return signals.Continue
	})

	isControllableState.AddCallback(func(newValue bool) {
		schwifty.OnMainThreadOncePure(func() {
			if newValue {
				actualPlayPauseChildState.SetValue(playPauseChildState.Value())
			} else {
				actualPlayPauseChildState.SetValue(playSpinner)
			}
		})
	})

	repeatClassState := state.NewStateful("")
	repeatIconState := state.NewStateful(repeatListIconName)
	repeatTooltipState := state.NewStateful("")
	player.RepeatModeChanged.On(func(rm player.RepeatMode) bool {
		switch rm {
		case player.RepeatModeNone:
			repeatClassState.SetValue("")
			repeatIconState.SetValue(repeatListIconName)
			repeatTooltipState.SetValue(gettext.Get("Repeat Queue"))
		case player.RepeatModeQueue:
			repeatClassState.SetValue("accent")
			repeatIconState.SetValue(repeatListIconName)
			repeatTooltipState.SetValue(gettext.Get("Repeat Track"))
		case player.RepeatModeTrack:
			repeatClassState.SetValue("accent")
			repeatIconState.SetValue(repeatTrackIconName)
			repeatTooltipState.SetValue(gettext.Get("Disable Repeat"))
		}
		return signals.Continue
	})

	shuffleClassState := state.NewStateful("")
	shuffleTooltipState := state.NewStateful("")
	player.ShuffleStateChanged.On(func(isShuffleEnabled bool) bool {
		if isShuffleEnabled {
			shuffleClassState.SetValue("accent")
			shuffleTooltipState.SetValue(gettext.Get("Disable Shuffle"))
		} else {
			shuffleClassState.SetValue("")
			shuffleTooltipState.SetValue(gettext.Get("Enable Shuffle"))
		}
		return signals.Continue
	})

	return HStack(
		controlButton.
			IconName("playlist-shuffle-symbolic").
			ActionName("win.player.shuffle").
			BindSensitive(isTrackLoadedState).
			BindTooltipText(shuffleTooltipState).
			BindCSSClass(shuffleClassState),
		Spacer().VExpand(false),
		controlButton.
			TooltipText(gettext.Get("Previous")).
			IconName("skip-backward-large-symbolic").
			BindSensitive(isTrackLoadedState).
			ActionName("win.player.previous"),
		Spacer().VExpand(false),
		Button().
			BindTooltipText(playPauseTooltipState).
			ActionName("win.player.play-pause").
			BindChild(actualPlayPauseChildState).
			BindSensitive(playPauseSensitiveState).
			WithCSSClass("suggested-action").CornerRadius(21).
			HPadding(32).VPadding(9),
		Spacer().VExpand(false),
		controlButton.
			TooltipText(gettext.Get("Next")).
			BindSensitive(isTrackLoadedState).
			IconName("skip-forward-large-symbolic").
			ActionName("win.player.next"),
		Spacer().VExpand(false),
		controlButton.
			BindTooltipText(repeatTooltipState).
			BindSensitive(isTrackLoadedState).
			BindIconName(repeatIconState).BindCSSClass(repeatClassState).
			ActionName("win.player.repeat"),
	).HExpand(true)
}
