package helper

import (
	"encoding/json"
	"time"

	"github.com/sosodev/duration"
)

type DurationISO8601 struct {
	time.Duration
}

func (r *DurationISO8601) MarshalJSON() ([]byte, error) {
	return json.Marshal(duration.Format(r.Duration))
}

func (d *DurationISO8601) UnmarshalJSON(b []byte) error {
	var s string
	err := json.Unmarshal(b, &s)
	if err != nil {
		return err
	}

	t, err := duration.Parse(s)
	if err != nil {
		return err
	}

	d.Duration = t.ToTimeDuration()
	return nil
}
