package gtk

import (
	"codeberg.org/puregotk/puregotk/v4/gdk"
	"codeberg.org/puregotk/puregotk/v4/gobject"
)

type Paintable interface {
	gdk.Paintable
	Ref() *gobject.Object
	Unref()
}
