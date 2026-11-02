package tracking

var Default = NewTracker()

func Track(ptr uintptr, objectType string) {
	Default.Track(ptr, objectType)
}

func TrackStack(ptr uintptr, stack []byte) {
	Default.TrackStack(ptr, stack)
}

func TrackGC(ptr uintptr) {
	Default.TrackGC(ptr)
}

func Untrack(ptr uintptr) {
	Default.Untrack(ptr)
}

func GetStatus(ptr uintptr) *TrackedObject {
	return Default.GetStatus(ptr)
}

func Alive() []*TrackedObject {
	return Default.Alive()
}
