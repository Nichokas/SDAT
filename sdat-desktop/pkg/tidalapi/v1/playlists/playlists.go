package playlists

import "codeberg.org/dergs/tonearm/pkg/tidalapi/internal"

type Playlists struct {
	client *internal.Client
}

func New(client *internal.Client) *Playlists {
	return &Playlists{
		client: client,
	}
}
