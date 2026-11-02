package albums

import "codeberg.org/dergs/tonearm/pkg/tidalapi/internal"

type Albums struct {
	client *internal.Client
}

func New(client *internal.Client) *Albums {
	return &Albums{
		client: client,
	}
}
