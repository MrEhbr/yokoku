-- Merges the positions and failures of media.scan_added and media.renumbered into media.scanner, at
-- the lower position; without both positions media.scanner starts from the beginning.
INSERT INTO subscriber_positions (subscriber, last_event_id)
SELECT 'media.scanner', MIN(last_event_id) FROM subscriber_positions
WHERE subscriber IN ('media.scan_added', 'media.renumbered')
HAVING COUNT(*) = 2;

UPDATE failed_deliveries SET subscriber = 'media.scanner'
WHERE subscriber IN ('media.scan_added', 'media.renumbered');

DELETE FROM subscriber_positions WHERE subscriber IN ('media.scan_added', 'media.renumbered');
