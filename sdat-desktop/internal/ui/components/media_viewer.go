package components

import (
	"time"

	"codeberg.org/dergs/tonearm/internal/ui/accels"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gdk"
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/gsk"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

type MediaViewer struct {
	revealer   *gtk.Revealer // darkened background + picture container
	picture    *gtk.Picture
	backButton *gtk.Button

	zoomLevel    float64
	baseW, baseH int32

	media *gdk.Texture
}

var (
	mediaViewer *MediaViewer
)

func (mv *MediaViewer) ToGTK() *gtk.Widget {
	return &mv.revealer.Widget
}

func GetMediaViewer() *MediaViewer {
	if mediaViewer != nil {
		return mediaViewer
	}

	mediaViewer = &MediaViewer{}

	clickGesture := gtk.NewGestureClick()
	clickGesture.ConnectPressed(new(func(_ gtk.GestureClick, _ int32, _ float64, _ float64) {
		mediaViewer.Hide()
	}))

	scrollGesture := gtk.NewEventControllerScroll(gtk.EventControllerScrollVerticalValue)
	scrollGesture.SetPropagationPhase(gtk.PhaseCaptureValue)
	scrollGesture.ConnectScroll(new(func(_ gtk.EventControllerScroll, _, dy float64) bool {
		// Only zoom when the primary modifier (Ctrl, or Cmd on macOS) is held
		state := scrollGesture.GetCurrentEventState()
		if state&(gdk.ControlMaskValue|gdk.MetaMaskValue) != 0 {
			if dy < 0 {
				go mediaViewer.zoom(1.1)
			} else {
				go mediaViewer.zoom(0.9)
			}
			return true // consume the event
		}
		return false
	}))

	zoomGesture := gtk.NewGestureZoom()
	var startZoom float64
	zoomGesture.ConnectBegin(new(func(_ gtk.Gesture, _ uintptr) {
		startZoom = mediaViewer.zoomLevel
	}))
	zoomGesture.ConnectScaleChanged(new(func(_ gtk.GestureZoom, scale float64) {
		go mediaViewer.setZoom(startZoom * scale)
	}))

	// --- Double-click on picture to reset zoom ---
	dblClick := gtk.NewGestureClick()
	dblClick.SetButton(1)
	dblClick.ConnectPressed(new(func(gesture gtk.GestureClick, nPress int32, _, _ float64) {
		gesture.SetState(gtk.EventSequenceClaimedValue)
		if nPress == 2 {
			mediaViewer.resetZoom()
		}
	}))

	backButton := Button().
		ActionName("mediaviewer.close").
		IconName("go-previous-symbolic").
		ConnectConstruct(func(b *gtk.Button) {
			mediaViewer.backButton = b
		})()

	model := gio.NewMenu()
	copyItem := gio.NewMenuItem("Copy Media", "mediaviewer.copy")
	model.AppendItem(copyItem)
	moreButton := MenuButton().
		MenuModel(&model.MenuModel).
		IconName("view-more-symbolic")
	toolbar := adw.NewToolbarView()
	toolbar.AddTopBar(HeaderBar().
		TitleWidget(Bin()).
		DecorationLayout("").
		PackStart(backButton).
		PackEnd(moreButton).
		ToGTK())
	toolbar.SetContent(
		ScrolledWindow().
			Child(
				AspectFrame(
					Picture().
						HAlign(gtk.AlignCenterValue).
						VAlign(gtk.AlignCenterValue).
						CanShrink(true).
						ContentFit(gtk.ContentFitContainValue).
						ConnectConstruct(func(p *gtk.Picture) {
							mediaViewer.picture = p
						}).
						Controller(&dblClick.EventController),
				),
			).
			HExpand(true).
			VExpand(true).
			Controller(&clickGesture.EventController).
			Controller(&scrollGesture.EventController).
			Controller(&zoomGesture.EventController).
			Policy(gtk.PolicyAutomaticValue, gtk.PolicyAutomaticValue).ToGTK(),
	)

	defer toolbar.Unref()

	overlay := adw.NewToastOverlay()
	overlay.SetChild(&toolbar.Widget)
	overlayRef := weak.NewWidgetRef(overlay)

	mediaViewer.revealer = Revealer(
		VStack(overlay).
			HAlign(gtk.AlignFillValue).
			VAlign(gtk.AlignFillValue).
			HExpand(true).
			VExpand(true).
			WithCSSClass("mediaviewer"),
	).
		ConnectConstruct(func(r *gtk.Revealer) {
			actionGroup := gio.NewSimpleActionGroup()
			defer actionGroup.Unref()

			copyItem := gio.NewSimpleAction("copy", nil)
			copyItem.ConnectActivate(new(func(action gio.SimpleAction, _ uintptr) {
				mediaViewer.copyMedia()

				overlayRef.Use(func(obj *gtk.Widget) {
					overlay := adw.ToastOverlayNewFromInternalPtr(obj.Ptr)
					toast := adw.NewToast("Copied image to clipboard")
					toast.SetTimeout(3)
					overlay.AddToast(toast)
				})
			}))
			actionGroup.AddAction(copyItem)

			closeItem := gio.NewSimpleAction("close", nil)
			closeItem.ConnectActivate(new(func(action gio.SimpleAction, _ uintptr) {
				mediaViewer.Hide()
			}))
			actionGroup.AddAction(closeItem)

			r.InsertActionGroup("mediaviewer", actionGroup)

			ShortcutController().
				ShortcutFromNames(accels.PrimaryMod+"C", "mediaviewer.copy").
				ShortcutFromNames("Escape", "mediaviewer.close").
				Into(r)
		}).
		ConnectShow(func(w gtk.Widget) {
			mediaViewer.backButton.GrabFocus()
		}).
		Visible(false).
		TransitionType(gtk.RevealerTransitionTypeCrossfadeValue)()

	return mediaViewer
}

func (mv *MediaViewer) copyMedia() {
	display := gdk.DisplayGetDefault()
	defer display.Unref()
	clipboard := display.GetClipboard()
	defer clipboard.Unref()

	clipboard.SetTexture(mv.media)
}

func (mv *MediaViewer) ShowFile(paintable gdk.Paintable) {
	if paintable == nil {
		return
	}

	mv.resetZoom()
	mv.picture.SetPaintable(paintable)
	mv.revealer.SetVisible(true)
	mv.revealer.SetRevealChild(true)

	snapshot := gtk.NewSnapshot()
	width := float64(paintable.GetIntrinsicWidth())
	height := float64(paintable.GetIntrinsicHeight())
	if width <= 0 {
		width = 1440
	}
	if height <= 0 {
		height = 1440
	}
	paintable.Snapshot(&snapshot.Snapshot, width, height)
	node := snapshot.FreeToNode()
	defer node.Unref()

	renderer := gsk.NewCairoRenderer()
	defer renderer.Unref()
	renderer.Realize(nil)
	defer renderer.Unrealize()
	mv.media = renderer.RenderTexture(node, nil)
}

func (mv *MediaViewer) Hide() {
	if !mv.revealer.GetRevealChild() {
		return
	}

	schwifty.OnMainThreadOncePure(func() {
		mv.revealer.SetRevealChild(false)
	})

	time.AfterFunc(time.Duration(mv.revealer.GetTransitionDuration()*uint32(time.Millisecond)), func() {
		schwifty.OnMainThreadOncePure(func() {
			mv.revealer.SetVisible(false)
		})
	})
}

func (mv *MediaViewer) resetZoom() {
	mv.zoomLevel = 1.0
	mv.baseW = 0 // force re-fetch of intrinsic size on next zoom
	mv.baseH = 0

	schwifty.OnMainThreadOncePure(func() {
		mv.picture.SetSizeRequest(-1, -1)
	})
}

func (mv *MediaViewer) zoom(factor float64) {
	mv.setZoom(mv.zoomLevel * factor)
}

func (mv *MediaViewer) setZoom(level float64) {
	// Clamp zoom between 0.5x and 5x
	if level < 1 {
		level = 1
	}
	if level > 10.0 {
		level = 10.0
	}
	mv.zoomLevel = level

	if mv.baseW == 0 || mv.baseH == 0 {
		c := make(chan struct{})
		schwifty.OnMainThreadOncePure(func() {
			mv.baseW = mv.picture.GetSize(gtk.OrientationHorizontalValue)
			mv.baseH = mv.picture.GetSize(gtk.OrientationVerticalValue)

			close(c)
		})

		<-c
	}

	w := int32(float64(mv.baseW) * mv.zoomLevel)
	h := int32(float64(mv.baseH) * mv.zoomLevel)
	schwifty.OnMainThreadOncePure(func() {
		mv.picture.SetSizeRequest(w, h)
	})
}
