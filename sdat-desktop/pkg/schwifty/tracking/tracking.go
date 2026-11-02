package tracking

import "sync"

type TrackedObject struct {
	IsReferencedByGo bool
	Type             string
	Stack            []byte
}

type Tracker struct {
	mu      sync.Mutex
	objects map[uintptr]*TrackedObject
}

func NewTracker() *Tracker {
	return &Tracker{
		objects: make(map[uintptr]*TrackedObject),
	}
}

func (t *Tracker) Track(ptr uintptr, objectType string) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.objects[ptr] = &TrackedObject{
		IsReferencedByGo: true,
		Type:             objectType,
	}
}

func (t *Tracker) TrackStack(ptr uintptr, stack []byte) {
	t.mu.Lock()
	defer t.mu.Unlock()
	if val, ok := t.objects[ptr]; ok {
		val.Stack = stack
	}
}

func (t *Tracker) TrackGC(ptr uintptr) {
	t.mu.Lock()
	defer t.mu.Unlock()
	if val, ok := t.objects[ptr]; ok {
		val.IsReferencedByGo = false
	}
}

func (t *Tracker) Untrack(ptr uintptr) {
	t.mu.Lock()
	defer t.mu.Unlock()
	delete(t.objects, ptr)
}

func (t *Tracker) GetStatus(ptr uintptr) *TrackedObject {
	t.mu.Lock()
	defer t.mu.Unlock()
	return t.objects[ptr]
}

func (t *Tracker) Alive() []*TrackedObject {
	t.mu.Lock()
	defer t.mu.Unlock()
	alive := make([]*TrackedObject, 0, len(t.objects))
	for _, object := range t.objects {
		alive = append(alive, object)
	}
	return alive
}
