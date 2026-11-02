package artists

import "codeberg.org/dergs/tonearm/pkg/tidalapi/internal"

type Artists struct {
	client *internal.Client
}

func New(client *internal.Client) *Artists {
	return &Artists{
		client: client,
	}
}
