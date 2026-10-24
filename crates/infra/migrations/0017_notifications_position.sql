-- Starts the notifications subscriber after the events already in the log; with none, it starts
-- from the beginning.
INSERT INTO subscriber_positions (subscriber, last_event_id)
SELECT 'notifications', MAX(id) FROM events
HAVING COUNT(*) > 0;
