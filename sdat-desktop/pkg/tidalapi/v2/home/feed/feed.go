package feed

import "codeberg.org/dergs/tonearm/pkg/tidalapi/internal"

type Feed struct {
	client *internal.Client
}

func New(client *internal.Client) *Feed {
	return &Feed{
		client: client,
	}
}
