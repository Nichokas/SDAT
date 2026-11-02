package tracklist_header

import (
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"codeberg.org/puregotk/puregotk/v4/pango"
)

var playerControlsAvailableState = state.NewStateful(false)

func init() {
	player.PlaybackStateChanged.On(func(ps player.PlaybackState) bool {
		playerControlsAvailableState.SetValue(!ps.Loading)
		return signals.Continue
	})
}

func template(cover tonearm.Cover, title string, subtitle string, description string, controls schwifty.Box, secondaryControls schwifty.Box) schwifty.Widget {
	layout := adw.NewMultiLayoutView()
	defer layout.Unref()

	layout.SetChild("cover", componentCover(cover).ToGTK())
	layout.SetChild("title", Label(title).WithCSSClass("title-2").Ellipsis(pango.EllipsizeEndValue).HAlign(gtk.AlignStartValue).ToGTK())
	layout.SetChild("subtitle", Label(subtitle).Ellipsis(pango.EllipsizeEndValue).WithCSSClass("heading").WithCSSClass("dimmed").HAlign(gtk.AlignStartValue).ToGTK())
	layout.SetChild("controls", controls.ToGTK())
	layout.SetChild("secondaryControls", secondaryControls.ToGTK())

	layout.AddLayout(layoutDesktop(description))
	layout.AddLayout(layoutMobile(description))

	breakpointBin := adw.NewBreakpointBin()
	breakpointBin.SetChild(&layout.Widget)
	breakpointBin.SetSizeRequest(150, 150)

	breakpoint := adw.NewBreakpoint(adw.BreakpointConditionParse("max-width: 600px"))
	breakpoint.AddSetters(&layout.Object, "layout-name", "mobile")
	breakpointBin.AddBreakpoint(breakpoint)

	return ManagedWidget(&breakpointBin.Widget)
}

func layoutDesktop(description string) *adw.Layout {
	child := HStack(
		ManagedWidget(&adw.NewLayoutSlot("cover").Widget),
		VStack(
			// Title + Subtitle / Main Playback Controls
			HStack(
				VStack(
					ManagedWidget(&adw.NewLayoutSlot("title").Widget),
					ManagedWidget(&adw.NewLayoutSlot("subtitle").Widget),
				).HAlign(gtk.AlignStartValue),
				ManagedWidget(&adw.NewLayoutSlot("controls").Widget),
			),
			// Description / Secondary Controls
			HStack(
				Label(description).Wrap(true).Lines(3).Ellipsis(pango.EllipsizeEndValue).WithCSSClass("dimmed").HAlign(gtk.AlignStartValue).HExpand(true),
				ManagedWidget(&adw.NewLayoutSlot("secondaryControls").Widget).MarginStart(10),
			).MarginTop(20),
		).MarginStart(20).VAlign(gtk.AlignCenterValue),
	).WithCSSClass("tracklist_header").HExpand(true).ToGTK()

	layout := adw.NewLayout(child)
	layout.SetName("desktop")
	return layout
}

func layoutMobile(description string) *adw.Layout {
	child := VStack(
		ManagedWidget(&adw.NewLayoutSlot("cover").Widget).HAlign(gtk.AlignCenterValue),
		VStack(
			ManagedWidget(&adw.NewLayoutSlot("title").Widget).HAlign(gtk.AlignCenterValue),
			ManagedWidget(&adw.NewLayoutSlot("subtitle").Widget).HAlign(gtk.AlignCenterValue),
		).HAlign(gtk.AlignCenterValue).MarginTop(10),
		Label(description).Justify(gtk.JustifyCenterValue).Wrap(true).Lines(3).Ellipsis(pango.EllipsizeEndValue).WithCSSClass("dimmed").HAlign(gtk.AlignCenterValue).HExpand(true).SizeRequest(-1, 54),
		ManagedWidget(&adw.NewLayoutSlot("controls").Widget).HAlign(gtk.AlignCenterValue),
		ManagedWidget(&adw.NewLayoutSlot("secondaryControls").Widget).HAlign(gtk.AlignCenterValue),
	).Spacing(10).WithCSSClass("tracklist_header").HExpand(true).ToGTK()

	layout := adw.NewLayout(child)
	layout.SetName("mobile")
	return layout
}
