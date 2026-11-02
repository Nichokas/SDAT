package adw

import (
	"codeberg.org/puregotk/puregotk/v4/adw"
)

type ShortcutsItem func() *adw.ShortcutsItem

func (f ShortcutsItem) Accelerator(accelerator string) ShortcutsItem {
	return func() *adw.ShortcutsItem {
		page := f()
		page.SetAccelerator(accelerator)
		return page
	}
}

func (f ShortcutsItem) ActionName(actionName string) ShortcutsItem {
	return func() *adw.ShortcutsItem {
		page := f()
		page.SetActionName(actionName)
		return page
	}
}

func (f ShortcutsItem) Subtitle(subtitle string) ShortcutsItem {
	return func() *adw.ShortcutsItem {
		page := f()
		page.SetSubtitle(subtitle)
		return page
	}
}

func (f ShortcutsItem) Title(title string) ShortcutsItem {
	return func() *adw.ShortcutsItem {
		page := f()
		page.SetTitle(title)
		return page
	}
}
