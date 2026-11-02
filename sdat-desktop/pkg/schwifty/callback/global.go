package callback

import (
	"sync"

	"codeberg.org/puregotk/puregotk/v4/glib"
)

var callbackIdPool = NewIntPool()

type MainThreadCallback func(u uintptr) bool

type MainThreadCallbackEntry struct {
	Callback MainThreadCallback
	Param    uintptr
}

var mainThreadCallbacks = make(map[uintptr]MainThreadCallbackEntry)
var mainThreadCallbacksLock = sync.RWMutex{}

var mainLoopHandler = glib.SourceFunc(func(ptr uintptr) bool {
	mainThreadCallbacksLock.RLock()
	entry, ok := mainThreadCallbacks[ptr]
	if !ok {
		mainThreadCallbacksLock.RUnlock()
		callbackIdPool.Return(int(ptr))
		return glib.SOURCE_REMOVE
	}
	mainThreadCallbacksLock.RUnlock()

	shouldContinue := entry.Callback(entry.Param)
	if !shouldContinue {
		mainThreadCallbacksLock.Lock()
		delete(mainThreadCallbacks, ptr)
		callbackIdPool.Return(int(ptr))
		mainThreadCallbacksLock.Unlock()
	}
	return shouldContinue
})

func OnMainThread(callback MainThreadCallback, params uintptr) uint32 {
	if glib.MainContextDefault().IsOwner() {
		callback(params)
		return 0
	}

	return ScheduleOnMainThread(callback, params)
}

func OnMainThreadOnce(cb func(u uintptr), param uintptr) uint32 {
	return OnMainThread(func(u uintptr) bool {
		cb(param)
		return glib.SOURCE_REMOVE
	}, param)
}

func OnMainThreadOncePure(cb func()) uint32 {
	return OnMainThreadOnce(func(uintptr) { cb() }, 0)
}

func ScheduleOnMainThread(callback MainThreadCallback, params uintptr) uint32 {
	id := uintptr(callbackIdPool.Get())
	mainThreadCallbacksLock.Lock()
	mainThreadCallbacks[id] = MainThreadCallbackEntry{
		Callback: callback,
		Param:    params,
	}
	mainThreadCallbacksLock.Unlock()

	return glib.IdleAdd(&mainLoopHandler, id)
}

func ScheduleOnMainThreadOnce(cb func(u uintptr), param uintptr) uint32 {
	return ScheduleOnMainThread(func(u uintptr) bool {
		cb(param)
		return glib.SOURCE_REMOVE
	}, param)
}

func ScheduleOnMainThreadOncePure(cb func()) uint32 {
	return ScheduleOnMainThreadOnce(func(uintptr) { cb() }, 0)
}

// IntPool manages a pool of integers that can be checked out and returned
type IntPool struct {
	mu       sync.Mutex
	inUse    map[int]bool
	nextID   int
	returned []int
}

func NewIntPool() *IntPool {
	return &IntPool{
		inUse:    make(map[int]bool),
		nextID:   1,
		returned: make([]int, 0),
	}
}

func (p *IntPool) Get() int {
	p.mu.Lock()
	defer p.mu.Unlock()

	var id int

	// First try to reuse a returned ID
	if len(p.returned) > 0 {
		id = p.returned[len(p.returned)-1]
		p.returned = p.returned[:len(p.returned)-1]
	} else {
		// Otherwise use the next available ID
		id = p.nextID
		p.nextID++
	}

	p.inUse[id] = true
	return id
}

func (p *IntPool) Return(id int) {
	p.mu.Lock()
	defer p.mu.Unlock()

	if p.inUse[id] {
		delete(p.inUse, id)
		p.returned = append(p.returned, id)
	}
}
