package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

type ShortcutController func() *gtk.ShortcutController

func (f ShortcutController) Into(controllable interface{ AddController(*gtk.EventController) }) {
	callback.OnMainThreadOncePure(func() {
		c := f()
		// AddController will take ownership of the object, so we need to increase the reference count
		// to account for schwifty object tracking
		c.Ref()
		controllable.AddController(&c.EventController)
	})
}

func (f ShortcutController) Shortcut(TriggerVar *gtk.ShortcutTrigger, ActionVar *gtk.ShortcutAction) ShortcutController {
	return func() *gtk.ShortcutController {
		c := f()
		shortcut := gtk.NewShortcut(TriggerVar, ActionVar)
		c.AddShortcut(shortcut)
		return c
	}
}

func (f ShortcutController) ShortcutFromNames(TriggerName string, ActionName string) ShortcutController {
	trigger := gtk.ShortcutTriggerParseString(TriggerName)
	action := gtk.NewNamedAction(ActionName)

	return f.Shortcut(trigger, &action.ShortcutAction)
}
