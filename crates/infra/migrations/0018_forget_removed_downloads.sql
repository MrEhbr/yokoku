-- Downloads whose torrent left the client are no longer kept.
DELETE FROM downloads WHERE state = 'removed';
