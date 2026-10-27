# Desktop notifications (Rust notifier). Calm and actionable (design-language.md §8).
#
# $limit: five_hour | weekly | scoped. $name: the model of a scoped weekly limit, e.g. "Fable".

# $what is notify-limit formatted for the same limit (Fluent terms cannot take variables).

notify-limit = { $limit ->
    [weekly] Weekly limit
    [scoped] { $name } weekly limit
   *[five_hour] 5-hour window
}

## Threshold. $pct is the threshold the limit reached.

notify-threshold-title = { $pct ->
    [100] { $what } is full
   *[other] { $what } reached { PCT($pct) }
}

## Projection. $eta is the time until full, in ms.

notify-projection-title = At this rate the { $limit ->
    [weekly] weekly limit
    [scoped] { $name } weekly limit
   *[five_hour] 5-hour window
} fills in { DUR($eta) }
notify-final-title = { $what } fills in { DUR($eta) }

## When the limit resets. $ms is a duration; $at a weekday and time, e.g. "Wed 10:59".

notify-resets-in = Resets in { DUR($ms) }.
notify-resets-at = Resets { $at }.

## Reset

notify-reset-title = { $what } has reset
notify-reset-body = Full capacity again.

## After a mute or quiet hours: several notices in one. The body lists their titles.

notify-summary-title = { $count } notifications while paused

## "Send test notification" in the settings.

notify-test-title = TokenRadar test notification
notify-test-body = Notifications work. Limit alerts will look like this.
