package notifications

import "codeberg.org/dergs/tonearm/internal/signals"

var OnToast = signals.NewStatelessSignal[string]()
