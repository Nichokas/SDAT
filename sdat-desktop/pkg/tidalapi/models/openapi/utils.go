package openapi

import (
	"encoding/json"
	"fmt"
	"time"

	"github.com/sosodev/duration"
)

type DateTime struct {
	time.Time
}

func (r *DateTime) MarshalJSON() ([]byte, error) {
	return json.Marshal(r.Time.Format("2006-01-02T15:04:05.000Z"))
}

func (d *DateTime) UnmarshalJSON(b []byte) error {
	var s string
	err := json.Unmarshal(b, &s)
	if err != nil {
		return err
	}

	t, err := time.Parse("2006-01-02T15:04:05.000Z", s)
	if err != nil {
		t2, err := time.Parse("2006-01-02T15:04:05Z", s)
		if err != nil {
			fmt.Println(err)
			return err
		}
		t = t2
	}

	d.Time = t
	return nil
}

type Duration struct {
	time.Duration
}

func (r *Duration) MarshalJSON() ([]byte, error) {
	return json.Marshal(duration.Format(r.Duration))
}

func (d *Duration) UnmarshalJSON(b []byte) error {
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
