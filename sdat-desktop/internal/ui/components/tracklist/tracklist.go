package tracklist

import (
	"math"
	"slices"
	"sync"
	"unsafe"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/factory"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/schwifty/tracking"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gdk"
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/gobject"
	"codeberg.org/puregotk/puregotk/v4/graphene"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

type ColumnFunc func(track tonearm.Track, grid *gtk.Grid, position int, column int32) int

type TrackList struct {
	schwifty.Widget

	columnFuncs []ColumnFunc
	lock        sync.Mutex
	sizeGroups  []*gtk.SizeGroup
	store       *gio.ListStore
	trackList   []tonearm.Track

	clickHandler func(track tonearm.Track, position int)

	trackBeingMoved   tonearm.Track
	movingSourceIndex int
	movingTargetIndex int
	reorderable       *state.State[bool]
	reorderCallback   func(sourceIndex, targetIndex int, track tonearm.Track)
}

func (t *TrackList) AddTrack(track tonearm.Track) {
	t.trackList = append(t.trackList, track)
	t.store.Append(&gtk.NewStringObject("").Object)
}

func (t *TrackList) BindTracks(state *state.State[[]tonearm.Track]) {
	id := state.AddCallback(func(newValue []tonearm.Track) {
		t.lock.Lock()
		defer t.lock.Unlock()

		for i, newTrack := range newValue {
			if i >= len(t.trackList) {
				t.trackList = append(t.trackList, newTrack)
				t.store.Append(&gtk.NewStringObject("").Object)
			} else if t.trackList[i] != newTrack {
				t.trackList[i] = newTrack
				t.store.Splice(uint32(i), 1, []gobject.Object{gtk.NewStringObject("").Object}, 1)
			}
		}
		for i := len(t.trackList) - 1; i+1 > len(newValue); i-- {
			t.trackList = t.trackList[:i]
			t.store.Remove(uint32(i))
		}
	})
	t.ConnectDestroy(func(w gtk.Widget) {
		state.RemoveCallback(id)
	})
}

func (t *TrackList) Clear() {
	t.store.RemoveAll()
	t.trackList = make([]tonearm.Track, 0)
}

func (t *TrackList) SetClickHandler(cb func(track tonearm.Track, position int)) {
	t.clickHandler = cb
}

func (t *TrackList) SetReorderCallback(cb func(sourceIndex, targetIndex int, track tonearm.Track)) {
	t.reorderCallback = cb
	t.reorderable.SetValue(cb != nil)
}

func (t *TrackList) onBind(_ gtk.SignalListItemFactory, listItem *gtk.ListItem) {
	track := t.trackList[listItem.GetPosition()]
	container := adw.BinNewFromInternalPtr(listItem.GetChild().GoPointer())
	defer container.Unref()

	body := HStack().ConnectRealize(func(b gtk.Widget) {
		var ref = weak.NewWidgetRef(&b)
		player.TrackChanged.On(func(t *player.TrackChangedEvent) bool {
			return signals.ContinueIf(
				ref.Use(func(obj *gtk.Widget) {
					if t != nil && track.ID() == t.ID() {
						obj.AddCssClass("playing")
					} else {
						obj.RemoveCssClass("playing")
					}
				}),
			)
		})
	})
	for i, columnFunc := range t.columnFuncs {
		grid := gtk.NewGrid()
		grid.SetValign(gtk.AlignCenterValue)
		defer grid.Unref()
		columnFunc(track, grid, int(listItem.GetPosition()), 0)

		t.sizeGroups[i].AddWidget(&grid.Widget)
		body = body.Append(grid)
	}

	listItem.SetActivatable(track.IsStreamable())
	container.SetSensitive(track.IsStreamable())
	if !track.IsStreamable() {
		container.SetTooltipText(gettext.Get("This track is not available for streaming"))
	} else {
		container.SetTooltipText("")
	}
	container.SetChild(body.ToGTK())
}

func (t *TrackList) onUnbind(_ gtk.SignalListItemFactory, listItem *gtk.ListItem) {
	container := adw.BinNewFromInternalPtr(listItem.GetChild().GoPointer())
	container.SetChild(nil)
	container.Unref()
}

func (t *TrackList) onSetup(_ gtk.SignalListItemFactory, listItem *gtk.ListItem) {
	container := adw.NewBin()
	defer container.Unref()

	var dragSource *gtk.DragSource
	t.reorderable.AddCallback(func(newValue bool) {
		if newValue && dragSource == nil {
			dragSource = DragSource().
				Actions(gdk.ActionMoveValue).
				ConnectPrepare(func(dragSource gtk.DragSource, x, y float64) gdk.ContentProvider {
					t.movingSourceIndex = int(listItem.GetPosition())
					t.movingTargetIndex = int(listItem.GetPosition())

					track := t.trackList[t.movingSourceIndex]
					t.trackBeingMoved = track
					return *gdk.NewContentProviderTyped(gobject.TypePointerVal, &track)
				}).
				ConnectDragBegin(func(dragSource gtk.DragSource, drag gdk.Drag) {
					drag.SetData("t", uintptr(unsafe.Pointer(t)))

					snapshot := gtk.NewSnapshot()
					p := container.Widget.GetParent()
					defer p.Unref()
					p.SnapshotChild(&container.Widget, snapshot)
					size := graphene.Size{}
					size.Init(
						float32(container.Widget.GetAllocatedWidth()),
						float32(container.Widget.GetAllocatedHeight()),
					)
					paintable := snapshot.ToPaintable(&size)

					dragSource.SetIcon(paintable, paintable.GetIntrinsicWidth()/2, paintable.GetIntrinsicHeight()/2)
				}).
				ConnectDragCancel(func(dragSource gtk.DragSource, drag gdk.Drag, reason gdk.DragCancelReason) bool {
					content := drag.GetContent()
					defer content.Unref()

					value := gobject.Value{}
					value.Init(gobject.TypePointerVal)
					content.GetValue(&value)

					t.trackList = append(t.trackList[:t.movingTargetIndex], t.trackList[t.movingTargetIndex+1:]...)
					t.store.Remove(uint32(t.movingTargetIndex))

					t.trackList = append(t.trackList[:t.movingSourceIndex], append([]tonearm.Track{t.trackBeingMoved}, t.trackList[t.movingSourceIndex:]...)...)
					t.store.Insert(uint32(t.movingSourceIndex), &gtk.NewStringObject("").Object)

					t.movingSourceIndex = -1
					t.movingTargetIndex = -1
					t.trackBeingMoved = nil

					return true
				})()
			container.AddController(&dragSource.EventController)
		} else if dragSource != nil {
			container.RemoveController(&dragSource.EventController)
			dragSource = nil
		}
	})

	listItem.SetChild(&container.Widget)
}

func NewTrackList(columnFuncs ...ColumnFunc) *TrackList {
	store := gio.NewListStore(gtk.StringObjectGLibType())
	tracking.SetFinalizer("ListStore", store)

	tracklist := &TrackList{
		columnFuncs: columnFuncs,
		sizeGroups:  []*gtk.SizeGroup{},
		store:       store,
		trackList:   make([]tonearm.Track, 0),
		clickHandler: func(track tonearm.Track, position int) {
			player.PlayTrack(track)
		},

		movingSourceIndex: -1,
		movingTargetIndex: -1,
		reorderable:       state.NewStateful(false),
	}

	for range columnFuncs {
		sizeGroup := gtk.NewSizeGroup(gtk.SizeGroupHorizontalValue)
		tracking.SetFinalizer("SizeGroup", sizeGroup)
		tracklist.sizeGroups = append(tracklist.sizeGroups, sizeGroup)
	}

	factory := factory.NewSignalListItemFactory().
		ConnectSetup(tracklist.onSetup).
		ConnectBind(tracklist.onBind).
		ConnectUnbind(tracklist.onUnbind)()

	listView := gtk.NewListView(gtk.NewNoSelection(tracklist.store), &factory.ListItemFactory)
	listView.SetSingleClickActivate(true)
	listView.SetOrientation(gtk.OrientationVerticalValue)

	listView.ConnectActivate(&callback.ListViewActivate)
	callback.HandleCallback(listView.Object, "activate", func(_ gtk.ListView, index uint32) {
		track := tracklist.trackList[index]
		tracklist.clickHandler(track, int(index))
	})

	var dropTarget *gtk.DropTargetAsync
	tracklist.reorderable.AddCallback(func(newValue bool) {
		if newValue && dropTarget == nil {
			dropTarget = DropTargetAsync(gdk.NewContentFormatsForGtype(gobject.TypePointerVal), gdk.ActionMoveValue).
				ConnectAccept(func(dropTarget gtk.DropTargetAsync, drop gdk.Drop) bool {
					drag := drop.GetDrag()
					defer drag.Unref()
					if drag.GetData("t") != uintptr(unsafe.Pointer(tracklist)) {
						return false
					}

					return true
				}).
				ConnectDragMotion(func(dropTarget gtk.DropTargetAsync, drop gdk.Drop, x, y float64) gdk.DragAction {
					height := listView.GetFirstChild().GetAllocatedHeight()
					i := int(math.Floor(y / float64(height)))

					if i != tracklist.movingTargetIndex {
						if i > len(tracklist.trackList) {
							return gdk.ActionNoneValue
						}
						trackList := slices.Clone(tracklist.trackList)
						if tracklist.movingTargetIndex != -1 {
							tracklist.store.Remove(uint32(tracklist.movingTargetIndex))
							trackList = append(tracklist.trackList[:tracklist.movingTargetIndex], tracklist.trackList[tracklist.movingTargetIndex+1:]...)
						}

						tracklist.trackList = append(trackList[:i], append([]tonearm.Track{tracklist.trackBeingMoved}, trackList[i:]...)...)
						tracklist.store.Insert(uint32(i), &gtk.NewStringObject("").Object)

						tracklist.movingTargetIndex = i
					}

					return gdk.ActionMoveValue
				}).
				ConnectDrop(func(dropTarget gtk.DropTargetAsync, drop gdk.Drop, x, y float64) bool {
					drop.Finish(gdk.ActionMoveValue)

					tracklist.reorderCallback(tracklist.movingSourceIndex, tracklist.movingTargetIndex, tracklist.trackBeingMoved)

					tracklist.movingSourceIndex = -1
					tracklist.movingTargetIndex = -1

					return true
				})()
			listView.AddController(&dropTarget.EventController)
		} else if dropTarget != nil {
			listView.RemoveController(&dropTarget.EventController)
			dropTarget = nil
		}
	})

	tracklist.Widget = ManagedWidget(&listView.Widget).WithCSSClass("tracklist")
	return tracklist
}
