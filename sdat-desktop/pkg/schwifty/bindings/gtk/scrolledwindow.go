package gtk

import (
	"sync"

	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen ScrolledWindow *gtk.ScrolledWindow gtk

func (f ScrolledWindow) BindChild(state *state.State[any]) ScrolledWindow {
	return func() *gtk.ScrolledWindow {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue any) {
				widget := ResolveWidget(newValue)
				if widget == nil {
					callback.OnMainThreadOncePure(func() {
						if obj := ref.Get(); obj != nil {
							defer obj.Unref()
							gtk.ScrolledWindowNewFromInternalPtr(obj.Ptr).SetChild(nil)
						}
					})
				} else {
					widget.Ref()
					callback.OnMainThreadOncePure(func() {
						defer widget.Unref()
						if obj := ref.Get(); obj != nil {
							defer obj.Unref()
							gtk.ScrolledWindowNewFromInternalPtr(obj.Ptr).SetChild(widget)
						}
					})
				}
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f ScrolledWindow) ConnectEdgeReached(cb func(gtk.ScrolledWindow, gtk.PositionType)) ScrolledWindow {
	return func() *gtk.ScrolledWindow {
		scrolledWindow := f()
		callback.HandleCallback(scrolledWindow.Object, "edge-reached", cb)
		return scrolledWindow
	}
}

func (f ScrolledWindow) ConnectReachEdgeSoon(edge gtk.PositionType, cb func() bool) ScrolledWindow {
	return func() *gtk.ScrolledWindow {
		scrolledWindow := f()
		var adj *gtk.Adjustment

		if edge == gtk.PosTopValue || edge == gtk.PosBottomValue {
			adj = scrolledWindow.GetVadjustment()
		} else if edge == gtk.PosLeftValue || edge == gtk.PosRightValue {
			adj = scrolledWindow.GetHadjustment()
		} else {
			panic("Invalid edge type")
		}

		mutex := sync.Mutex{}
		var (
			valueChangedSubscription = -1
			changedSubscription      = -1
		)

		unsub := func(adj *gtk.Adjustment) {
			if valueChangedSubscription >= 0 {
				callback.DeleteCallback(adj.Object, "value-changed", valueChangedSubscription)
				valueChangedSubscription = -1
			}
			if changedSubscription >= 0 {
				callback.DeleteCallback(adj.Object, "changed", changedSubscription)
				changedSubscription = -1
			}
		}

		shouldTrigger := func(adj *gtk.Adjustment) bool {
			if edge == gtk.PosTopValue || edge == gtk.PosLeftValue {
				return adj.GetValue() <= 0.2*adj.GetUpper()
			} else {
				return adj.GetValue()+adj.GetPageSize() >= 0.8*adj.GetUpper()
			}
		}

		adj.ConnectValueChanged(&callback.AdjustmentValueChangedCallback)
		valueChangedSubscription = callback.HandleCallback(adj.Object, "value-changed", func(adj gtk.Adjustment) {
			if !mutex.TryLock() {
				return
			}
			defer mutex.Unlock()

			if shouldTrigger(&adj) {
				go func() {
					mutex.Lock()
					defer mutex.Unlock()

					adj.Ref()
					defer adj.Unref()

					if cb() == signals.Unsubscribe {
						unsub(&adj)
					}
				}()
			}
		})

		adj.ConnectChanged(&callback.AdjustmentChangedCallback)
		changedSubscription = callback.HandleCallback(adj.Object, "changed", func(adj gtk.Adjustment) {
			if !mutex.TryLock() {
				return
			}
			defer mutex.Unlock()

			if adj.GetUpper() <= adj.GetPageSize() {
				go func() {
					mutex.Lock()
					adj.Ref()
					defer mutex.Unlock()
					defer adj.Unref()

					if cb() == signals.Unsubscribe {
						unsub(&adj)
					}
				}()
				return
			}

			if shouldTrigger(&adj) {
				go func() {
					mutex.Lock()
					adj.Ref()
					defer mutex.Unlock()
					defer adj.Unref()

					if cb() == signals.Unsubscribe {
						unsub(&adj)
					}
				}()
			}
		})

		adj.Unref()

		return scrolledWindow
	}
}

func (f ScrolledWindow) Child(widget any) ScrolledWindow {
	return func() *gtk.ScrolledWindow {
		scrolledWindow := f()
		scrolledWindow.SetChild(ResolveWidget(widget))
		return scrolledWindow
	}
}

func (f ScrolledWindow) Policy(hPolicy, vPolicy gtk.PolicyType) ScrolledWindow {
	return func() *gtk.ScrolledWindow {
		scrolledWindow := f()
		scrolledWindow.SetPolicy(hPolicy, vPolicy)
		return scrolledWindow
	}
}

func (f ScrolledWindow) PropagateNaturalHeight(propagate bool) ScrolledWindow {
	return func() *gtk.ScrolledWindow {
		scrolledWindow := f()
		scrolledWindow.SetPropagateNaturalHeight(propagate)
		return scrolledWindow
	}
}

func (f ScrolledWindow) PropagateNaturalWidth(propagate bool) ScrolledWindow {
	return func() *gtk.ScrolledWindow {
		scrolledWindow := f()
		scrolledWindow.SetPropagateNaturalWidth(propagate)
		return scrolledWindow
	}
}
