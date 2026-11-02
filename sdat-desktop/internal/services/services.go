package services

import (
	"codeberg.org/dergs/tonearm/internal/services/tidal"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"github.com/infinytum/injector"
)

func init() {
	injector.DeferredSingleton(func(api *tidalapi.TidalAPI) tonearm.Service {
		return tidal.NewTidal(api)
	})
}
