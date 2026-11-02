package horizontal_list

import (
	"math"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gobject"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"codeberg.org/puregotk/puregotk/v4/pango"
)

type HorizontalList struct {
	schwifty.Box

	container        *gtk.Box
	marginState      *state.State[int32]
	routeButtonState *state.State[any]
}

func (h *HorizontalList) Append(child schwifty.BaseWidgetable) *HorizontalList {
	h.container.Append(child.ToGTK())
	return h
}

func (h *HorizontalList) SetPageMargin(margin int32) *HorizontalList {
	h.marginState.SetValue(margin)
	return h
}

func (h *HorizontalList) SetViewAllRoute(path string) *HorizontalList {
	h.routeButtonState.SetValue(Button().Child(
		Label(gettext.Get("View All")).WithCSSClass("caption-heading"),
	).
		MinHeight(10).
		MinWidth(10).
		HPadding(10).
		ConnectClicked(func(b gtk.Button) {
			router.Navigate(path)
		}))
	return h
}

func NewHorizontalList(title string) *HorizontalList {
	marginState := state.NewStateful[int32](0)
	routeButtonState := state.NewStateful[any](nil)
	container := HStack().BindHMargin(marginState)()

	var hAdjustWeakRef weak.ObjectRef

	nextButton := Button().
		MinHeight(10).MinWidth(10).HPadding(10).
		Child(Image().FromIconName("right-symbolic").PixelSize(10)).
		TooltipText(gettext.Get("Scroll to the Right")).
		ConnectClicked(func(b gtk.Button) {
			hAdjustWeakRef.Use(func(obj *gobject.Object) {
				hAdjust := gtk.AdjustmentNewFromInternalPtr(obj.Ptr)

				current := hAdjust.GetValue()
				current -= math.Mod(current, 192)
				hAdjust.SetStepIncrement(192)
				go hAdjust.SetValue(current + hAdjust.GetStepIncrement())
			})
		})

	previousButton := Button().
		MinHeight(10).MinWidth(10).HPadding(10).
		Child(Image().FromIconName("left-symbolic").PixelSize(10)).
		TooltipText(gettext.Get("Scroll to the Left")).
		ConnectClicked(func(b gtk.Button) {
			hAdjustWeakRef.Use(func(obj *gobject.Object) {
				hAdjust := gtk.AdjustmentNewFromInternalPtr(obj.Ptr)

				current := hAdjust.GetValue()
				if math.Mod(current, 192) > 0 {
					current += 192 - math.Mod(current, 192)
				}
				hAdjust.SetStepIncrement(192)
				hAdjust.SetValue(current - hAdjust.GetStepIncrement())
			})
		})

	return &HorizontalList{
		Box: VStack(
			HStack(
				HStack(
					Label(title).
						WithCSSClass("title-2").
						VAlign(gtk.AlignCenterValue).
						Ellipsis(pango.EllipsizeEndValue),
					Spacer().VExpand(false),
					HStack(
						CenterBox().BindCenterWidget(routeButtonState).HExpand(false).VExpand(false),
						previousButton,
						nextButton,
					).Spacing(10),
				).BindHMargin(marginState),
			).HMargin(10).MarginBottom(5),
			ScrolledWindow().
				Child(container).
				VAlign(gtk.AlignStartValue).
				Policy(gtk.PolicyExternalValue, gtk.PolicyNeverValue).
				PropagateNaturalWidth(true).
				PropagateNaturalWidth(true).
				ConnectRealize(func(w gtk.Widget) {
					sw := gtk.ScrolledWindowNewFromInternalPtr(w.Ptr)
					child := sw.GetChild()
					defer child.Unref()
					child.SetOverflow(gtk.OverflowVisibleValue)

					adj := sw.GetHadjustment()
					defer adj.Unref()
					hAdjustWeakRef = weak.NewObjectRef(adj)
				}),
		),
		routeButtonState: routeButtonState,
		marginState:      marginState,
		container:        container,
	}
}
