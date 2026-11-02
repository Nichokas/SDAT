package signals

type StatelessSignal[T any] struct {
	Signal[func(T) bool]
}

func (s *StatelessSignal[T]) Notify(newValue T) {
	s.Signal.Notify(newValue)
}

func (s *StatelessSignal[T]) On(handler func(T) bool) *Subscription {
	return s.Signal.On(handler)
}

func NewStatelessSignal[T any]() *StatelessSignal[T] {
	return &StatelessSignal[T]{
		Signal: NewSignal[func(T) bool](),
	}
}
