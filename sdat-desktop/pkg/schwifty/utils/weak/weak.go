package weak

import (
	_ "unsafe"
)

// The Puregotk package does not export the raw C binding functions, but we *really* need to bypass
// their code as it is fundamentally unusable due to no allocator function being present for GWeakRef.
//
// Instead we handle the malloc / free ourselves and use the C bindings directly to manage the memory.

//go:linkname xWeakRefInit codeberg.org/puregotk/puregotk/v4/gobject.xWeakRefInit
var xWeakRefInit func(uintptr, uintptr)

//go:linkname xWeakRefClear codeberg.org/puregotk/puregotk/v4/gobject.xWeakRefClear
var xWeakRefClear func(uintptr)

//go:linkname xWeakRefGet codeberg.org/puregotk/puregotk/v4/gobject.xWeakRefGet
var xWeakRefGet func(uintptr) uintptr

//go:linkname xWeakRefSet codeberg.org/puregotk/puregotk/v4/gobject.xWeakRefSet
var xWeakRefSet func(uintptr, uintptr)

type weakRef[T gObject, Result gObject] interface {
	// Clear frees resources associated with a non-statically-allocated WeakRef.
	// After this call, the WeakRef is left in an undefined state.
	//
	// You should only call this on a WeakRef that previously had
	// Init called on it.
	Clear()

	// Get atomically acquires a strong reference to the object the WeakRef
	// points to, if it is not empty, and returns it.
	//
	// This is needed because of the potential race between taking the pointer
	// value and adding a reference to it, if the object was losing its last
	// reference at the same time in a different thread.
	//
	// The caller should release the resulting reference when done,
	// typically by calling Unref.
	Get() Result

	// Init initialises a non-statically-allocated WeakRef.
	//
	// This also calls Set with obj on the freshly-initialised weak reference.
	//
	// Init should always be matched with a call to Clear. It is not
	// necessary to use Init for a WeakRef in static storage because it
	// will already be properly initialised. Just use Set directly.
	Init(obj T)

	// Set changes the object to which the WeakRef points, or sets it to nil.
	//
	// You must own a strong reference on obj while calling this function.
	Set(obj T)

	// Use calls cb with the referenced object if the WeakRef is not empty.
	// It returns true if the callback was invoked (i.e. the object was still alive),
	// or false if the WeakRef was empty.
	Use(cb func(obj Result)) bool
}
