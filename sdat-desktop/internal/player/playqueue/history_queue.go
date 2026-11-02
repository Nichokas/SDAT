package playqueue

import "codeberg.org/dergs/tonearm/pkg/tonearm"

type HistoryQueue struct {
	Queue
	Pending tonearm.Track
}

func (h *HistoryQueue) Prepend(track tonearm.Track) {
	if h.Pending != nil {
		h.Queue.Prepend(h.Pending)
	}
	h.Pending = track
}

func (h *HistoryQueue) Pop() tonearm.Track {
	h.Pending = nil
	return h.Queue.Pop()
}

func NewHistoryQueue() *HistoryQueue {
	return &HistoryQueue{
		Queue: NewQueue(),
	}
}
