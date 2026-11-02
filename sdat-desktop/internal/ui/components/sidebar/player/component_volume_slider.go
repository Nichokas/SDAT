package player

import (
	"math"

	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

var volumeState = state.NewStateful(1.0)

func init() {
	player.VolumeChanged.On(func(volume float64) bool {
		schwifty.OnMainThreadOncePure(func() {
			// Cube root the volume to account for the logarithmic nature of human volume perception
			volumeState.SetValue(math.Pow(volume, 1.0/3.0))
		})
		return signals.Continue
	})
}

func controlsVolumeSlider() schwifty.Popover {
	return Popover(
		Scale(gtk.OrientationVerticalValue).
			Inverted(true).
			Range(0, 1).
			MinHeight(100).
			HExpand(true).
			CSS(`scale:active { background-color: transparent; }`).
			BindValue(volumeState).
			ConnectChangeValue(func(r gtk.Range, st gtk.ScrollType, value float64) bool {
				// Cube the value to account for the logarithmic nature of human volume perception
				player.SetVolume(math.Pow(value, 3))
				return false
			}),
	)
}
