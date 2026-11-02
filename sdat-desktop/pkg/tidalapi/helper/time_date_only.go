package helper

import (
	"encoding/json"
	"time"
)

type TimeDateOnly struct {
	time.Time
}

func (r *TimeDateOnly) MarshalJSON() ([]byte, error) {
	return json.Marshal(r.Time.Format(time.DateOnly))
}

func (d *TimeDateOnly) UnmarshalJSON(b []byte) error {
	var s string
	err := json.Unmarshal(b, &s)
	if err != nil {
		return err
	}

	t, err := time.Parse(time.DateOnly, s)
	if err != nil {
		return err
	}

	d.Time = t
	return nil
}
