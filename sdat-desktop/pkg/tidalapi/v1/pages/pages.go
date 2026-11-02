package pages

import "codeberg.org/dergs/tonearm/pkg/tidalapi/internal"

type Pages struct {
	client *internal.Client
}

func New(client *internal.Client) *Pages {
	return &Pages{
		client: client,
	}
}
