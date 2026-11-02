package secrets

import (
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
)

var SignedInChanged *signals.StatefulSignal[bool]
var SignedInState *state.State[bool]

func init() {
	hasToken := HasRefreshToken()
	SignedInState = state.NewStateful(hasToken)
	SignedInChanged = signals.NewStatefulSignal(hasToken)
	SignedInChanged.On(func(b bool) bool {
		SignedInState.SetValue(b)
		return signals.Continue
	})
}

func triggerSignedInChanged() {
	SignedInChanged.Notify(func(oldValue bool) bool {
		return HasRefreshToken()
	})
}
