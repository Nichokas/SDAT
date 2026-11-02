package tracks

import "codeberg.org/dergs/tonearm/pkg/tidalapi/internal"

type Tracks struct {
	client *internal.Client
}

func New(client *internal.Client) *Tracks {
	return &Tracks{
		client: client,
	}
}
