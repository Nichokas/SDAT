package shortcut_list

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/puregotk/puregotk/v4/adw"
)

type ShortcutList struct {
	schwifty.Widget

	container *adw.WrapBox
}

func (f *ShortcutList) Append(widget schwifty.BaseWidgetable) {
	f.container.Append(widget.ToGTK())
}

func NewShortcutList() *ShortcutList {
	container := WrapBox().
		ChildSpacing(10).
		LineSpacing(10)()
	return &ShortcutList{
		Widget:    Widget(&container.Widget),
		container: container,
	}
}
