package player

import (
	"fmt"

	"codeberg.org/dergs/tonearm/internal/g"
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/notifications"
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gdk"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

var (
	durationState       = state.NewStateful("00:00")
	positionState       = state.NewStateful("00:00")
	timelineSliderState = state.NewStateful(0.0)

	selectedQuality      = state.NewStateful("Max")
	actualQuality        = state.NewStateful("N/A")
	playbackQualityText  = state.NewBoundStateful(selectedQuality)
	areDetailsDisplayed  = false
	playbackQualityClass = state.NewStateful("max")
)

func init() {
	player.TrackChanged.On(func(trackInfo *player.TrackChangedEvent) bool {
		schwifty.OnMainThreadOncePure(func() {
			if trackInfo == nil {
				durationState.SetValue("00:00")
			} else {
				durationState.SetValue(tidalapi.FormatDuration(trackInfo.Duration()))
			}
		})
		return signals.Continue
	})

	player.PlaybackStateChanged.On(func(state player.PlaybackState) bool {
		schwifty.OnMainThreadOncePure(func() {
			positionState.SetValue(tidalapi.FormatDuration(state.Position))
			if state.Duration > 0 {
				durationState.SetValue(tidalapi.FormatDuration(state.Duration))
				timelineSliderState.SetValue(100.0 / float64(state.Duration) * float64(state.Position))
			} else {
				timelineSliderState.SetValue(0)
			}
		})
		return signals.Continue
	})

	player.PlaybackQualityChanged.On(func(quality tonearm.TrackQuality) bool {
		setSelectedQualityLabel(quality)
		return signals.Continue
	})

	settings.Player().ConnectAudioQualityChanged(func(aq tonearm.TrackQuality) bool {
		notifications.OnToast.Notify(gettext.Get("Playback quality saved. Changes will be applied on next track change"))

		return signals.Continue
	})

	player.AudioStreamQuality.On(func(sq player.StreamQuality) bool {
		switch sq.Codec {
		case player.CodecAAC:
			bitrate := g.TruncateFloat(float64(sq.BitRate)/1000, 1)
			actualQuality.SetValue(fmt.Sprintf("%s kbps AAC", bitrate))
		case player.CodecFLAC:
			sampleRate := g.TruncateFloat(float64(sq.SampleRate)/1000, 1)
			actualQuality.SetValue(fmt.Sprintf("%d-bit %skHz FLAC", sq.BitDepth, sampleRate))
		default:
			actualQuality.SetValue("N/A")
		}
		return signals.Continue
	})

	setSelectedQualityLabel(settings.Player().GetAudioQuality())
}

func hideCheckmarkHook(quality tonearm.TrackQuality) func(*gtk.Label) {
	return func(l *gtk.Label) {
		settings.Player().ConnectAudioQualityChanged(
			func(aq tonearm.TrackQuality) bool {
				if aq == quality {
					l.Show()
				} else {
					l.Hide()
				}

				return signals.Continue
			},
		)

		if settings.Player().GetAudioQuality() == quality {
			l.Show()
		} else {
			l.Hide()
		}
	}
}

func makeQualitySelectEntry(quality tonearm.TrackQuality, css, label, details string, popover *weak.WidgetRef) schwifty.Button {
	return Button().
		Child(
			HStack(
				VStack(
					Label(label).WithCSSClass("heading").HAlign(gtk.AlignStartValue),
					Label(details).WithCSSClass("caption").HAlign(gtk.AlignStartValue),
				),
				Spacer(),
				Label("✓").ConnectConstruct(hideCheckmarkHook(quality)),
			).Spacing(10),
		).
		ConnectClicked(func(b gtk.Button) {
			settings.Player().SetAudioQuality(quality)
			(*popover).Use(func(obj *gtk.Widget) {
				popover := gtk.PopoverNewFromInternalPtr(obj.Ptr)
				popover.Hide()
			})
		}).
		WithCSSClass(css)
}

func setSelectedQualityLabel(quality tonearm.TrackQuality) {
	switch quality {
	case tonearm.TrackQualityLossy:
		selectedQuality.SetValue(gettext.Get("Low (96 kbps)"))
		playbackQualityClass.SetValue("low")
	case tonearm.TrackQualityHigh:
		selectedQuality.SetValue(gettext.Get("Low (320 kbps)"))
		playbackQualityClass.SetValue("low")
	case tonearm.TrackQualityLossless:
		selectedQuality.SetValue(gettext.Get("High"))
		playbackQualityClass.SetValue("high")
	case tonearm.TrackQualityHighResLossless:
		selectedQuality.SetValue(gettext.Get("Max"))
		playbackQualityClass.SetValue("max")
	}
}

func toggleQualityLabel() {
	if areDetailsDisplayed {
		playbackQualityText.BindState(selectedQuality)
		areDetailsDisplayed = false
	} else {
		playbackQualityText.BindState(actualQuality)
		areDetailsDisplayed = true
	}
}

func trackTimeline() schwifty.Widget {
	overlay := gtk.NewOverlay()
	overlay.SetChild(VStack(
		Scale(gtk.OrientationHorizontalValue).
			BindSensitive(isControllableState).
			BindValue(timelineSliderState).
			Increments(1, 1).
			HExpand(true).
			Range(0, 100).
			Background("transparent").
			HPadding(0).
			ConnectChangeValue(func(r gtk.Range, st gtk.ScrollType, f float64) bool {
				player.SeekToPercent(f)
				return false
			}),
		HStack(
			Label("").BindText(positionState),
			Spacer().VExpand(false),
			Label("").BindText(durationState),
		),
	).MarginBottom(2).ToGTK())

	var ref weak.WidgetRef
	popover := Popover(
		VStack(
			makeQualitySelectEntry(tonearm.TrackQualityLossy, "low", "Low (96 kbps)", "96 kbps AAC", &ref),
			makeQualitySelectEntry(tonearm.TrackQualityHigh, "low", "Low (320 kbps)", "320 kbps AAC", &ref),
			makeQualitySelectEntry(tonearm.TrackQualityLossless, "high", "High", "16-bit 44.1kHz FLAC", &ref),
			makeQualitySelectEntry(tonearm.TrackQualityHighResLossless, "max", "Max", "24-bit 192kHz FLAC", &ref),
		).
			WithCSSClass("selector").
			Spacing(8),
	)()
	ref = weak.NewWidgetRef(popover)

	overlay.AddOverlay(
		MenuButton().
			WithCSSClass("quality-selector").
			Child(
				Label("").
					WithCSSClass("caption-heading").
					BindText(playbackQualityText).
					BindCSSClass(playbackQualityClass).
					CornerRadius(10).
					HPadding(8).
					VPadding(4),
			).
			ConnectConstruct(func(mb *gtk.MenuButton) {
				ref := weak.NewWidgetRef(mb)

				click := gtk.NewGestureClick()
				click.SetButton(3)
				click.ConnectPressed(new(func(click gtk.GestureClick, nPress int32, x, y float64) {
					toggleQualityLabel()
				}))
				mb.AddController(&click.EventController)

				longPress := gtk.NewGestureLongPress()
				var longPressed bool
				longPress.ConnectPressed(new(func(gtk.GestureLongPress, float64, float64) {
					longPressed = true
					toggleQualityLabel()
				}))
				longPress.ConnectEnd(new(func(gtk.Gesture, uintptr) {
					if longPressed {
						ref.Use(func(obj *gtk.Widget) {
							mb := gtk.MenuButtonNewFromInternalPtr(obj.Ptr)
							mb.SetActive(false)
						})
					}
					longPressed = false
				}))
				mb.AddController(&longPress.EventController)

				keyCtrl := gtk.NewEventControllerKey()
				keyCtrl.SetPropagationPhase(gtk.PhaseCaptureValue)
				keyCtrl.ConnectKeyPressed(new(func(ctrl gtk.EventControllerKey, keyval uint32, keycode uint32, state gdk.ModifierType) bool {
					switch int32(keyval) {
					case gdk.KEY_space, gdk.KEY_KP_Space:
						toggleQualityLabel()
						return gdk.EVENT_STOP
					}

					return gdk.EVENT_PROPAGATE // propagate unhandled keys
				}))
				mb.AddController(&keyCtrl.EventController)
			}).
			Popover(popover).
			HExpand(false).
			VAlign(gtk.AlignEndValue).
			HAlign(gtk.AlignCenterValue).
			ToGTK(),
	)

	return ManagedWidget(&overlay.Widget)
}
