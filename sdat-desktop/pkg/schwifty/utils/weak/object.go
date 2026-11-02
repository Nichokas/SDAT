package weak

import (
	"runtime"
	"unsafe"

	"codeberg.org/puregotk/puregotk/v4/glib"
	"codeberg.org/puregotk/puregotk/v4/gobject"
)

type gObject interface {
	GoPointer() uintptr
	Unref()
}

type ObjectRef = weakRef[gObject, *gobject.Object]

type objectRef struct {
	gWeakRefMemoryPtr uintptr
}

func (x *objectRef) Clear() {
	if x.gWeakRefMemoryPtr == 0 {
		return
	}

	xWeakRefClear(x.gWeakRefMemoryPtr)
}

func (x *objectRef) Get() *gobject.Object {
	if x.gWeakRefMemoryPtr == 0 {
		return nil
	}

	ref := xWeakRefGet(x.gWeakRefMemoryPtr)
	if ref == 0 {
		return nil
	}

	return &gobject.Object{
		Ptr: ref,
	}
}

func (x *objectRef) Init(obj gObject) {
	if x.gWeakRefMemoryPtr == 0 {
		return
	}

	if obj == nil {
		xWeakRefInit(x.gWeakRefMemoryPtr, 0)
	} else {
		xWeakRefInit(x.gWeakRefMemoryPtr, obj.GoPointer())
	}
}

func (x *objectRef) Set(obj gObject) {
	if x.gWeakRefMemoryPtr == 0 {
		return
	}

	if obj == nil {
		xWeakRefSet(x.gWeakRefMemoryPtr, 0)
	} else {
		xWeakRefSet(x.gWeakRefMemoryPtr, obj.GoPointer())
	}
}

func (x *objectRef) Use(cb func(obj *gobject.Object)) bool {
	if x.gWeakRefMemoryPtr == 0 {
		return false
	}

	if obj := x.Get(); obj != nil {
		cb(obj)
		obj.Unref()
		return true
	}
	return false
}

func NewObjectRef[T gObject](obj T) ObjectRef {
	x := &objectRef{
		gWeakRefMemoryPtr: glib.Malloc(uint(unsafe.Sizeof(uintptr(0)) * 2)),
	}
	xWeakRefInit(x.gWeakRefMemoryPtr, obj.GoPointer())

	runtime.AddCleanup(x, func(ptr uintptr) {
		xWeakRefClear(ptr)
		glib.Free(ptr)
	}, x.gWeakRefMemoryPtr)

	return x
}
