use crate::neighbors::Direction;
use crate::{Coordinate, GeohashError, Neighbors, Rect};

static BASE32_CODES: &[char] = &[
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'j', 'k',
    'm', 'n', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
];

/// Encode a coordinate to a geohash with length `len`.
///
/// ### Examples
///
/// Encoding a coordinate to a length five geohash:
///
/// ```rust
/// let coord = geohash::Coordinate { x: -120.6623, y: 35.3003 };
///
/// let geohash_string = geohash::encode(coord, 5).expect("Invalid coordinate");
///
/// assert_eq!(geohash_string, "9q60y");
/// ```
///
/// Encoding a coordinate to a length ten geohash:
///
/// ```rust
/// let coord = geohash::Coordinate { x: -120.6623, y: 35.3003 };
///
/// let geohash_string = geohash::encode(coord, 10).expect("Invalid coordinate");
///
/// assert_eq!(geohash_string, "9q60y60rhs");
/// ```
pub fn encode(c: Coordinate<f64>, len: usize) -> Result<String, GeohashError> {
    let mut out = String::with_capacity(len);

    let mut bits_total: i8 = 0;
    let mut hash_value: usize = 0;
    let mut max_lat = 90f64;
    let mut min_lat = -90f64;
    let mut max_lon = 180f64;
    let mut min_lon = -180f64;

    if c.x < min_lon || c.x > max_lon || c.y < min_lat || c.y > max_lat {
        return Err(GeohashError::InvalidCoordinateRange(c));
    }

    while out.len() < len {
        for _ in 0..5 {
            if bits_total % 2 == 0 {
                let mid = (max_lon + min_lon) / 2f64;
                if c.x > mid {
                    hash_value = (hash_value << 1) + 1usize;
                    min_lon = mid;
                } else {
                    hash_value <<= 1;
                    max_lon = mid;
                }
            } else {
                let mid = (max_lat + min_lat) / 2f64;
                if c.y > mid {
                    hash_value = (hash_value << 1) + 1usize;
                    min_lat = mid;
                } else {
                    hash_value <<= 1;
                    max_lat = mid;
                }
            }
            bits_total += 1;
        }

        let code: char = BASE32_CODES[hash_value];
        out.push(code);
        hash_value = 0;
    }
    Ok(out)
}

/// Decode geohash string into latitude, longitude
///
/// Parameters:
/// Geohash encoded `&str`
///
/// Returns:
/// A four-element tuple describs a bound box:
/// * min_lat
/// * max_lat
/// * min_lon
/// * max_lon
pub fn decode_bbox(hash_str: &str) -> Result<Rect<f64>, GeohashError> {
    let mut is_lon = true;
    let mut max_lat = 90f64;
    let mut min_lat = -90f64;
    let mut max_lon = 180f64;
    let mut min_lon = -180f64;
    let mut mid: f64;
    let mut hash_value: usize;

    for c in hash_str.chars() {
        hash_value = hash_value_of_char(c)?;

        for bs in 0..5 {
            let bit = (hash_value >> (4 - bs)) & 1usize;
            if is_lon {
                mid = (max_lon + min_lon) / 2f64;

                if bit == 1 {
                    min_lon = mid;
                } else {
                    max_lon = mid;
                }
            } else {
                mid = (max_lat + min_lat) / 2f64;

                if bit == 1 {
                    min_lat = mid;
                } else {
                    max_lat = mid;
                }
            }
            is_lon = !is_lon;
        }
    }

    Ok(Rect::new(
        Coordinate {
            x: min_lon,
            y: min_lat,
        },
        Coordinate {
            x: max_lon,
            y: max_lat,
        },
    ))
}

fn hash_value_of_char(c: char) -> Result<usize, GeohashError> {
    let ord = c as usize;
    if (48..=57).contains(&ord) {
        return Ok(ord - 48);
    } else if (98..=104).contains(&ord) {
        return Ok(ord - 88);
    } else if (106..=107).contains(&ord) {
        return Ok(ord - 89);
    } else if (109..=110).contains(&ord) {
        return Ok(ord - 90);
    } else if (112..=122).contains(&ord) {
        return Ok(ord - 91);
    }
    Err(GeohashError::InvalidHashCharacter(c))
}

/// Decode a geohash into a coordinate with some longitude/latitude error. The
/// return value is `(<coordinate>, <longitude error>, <latitude error>)`.
///
/// ### Examples
///
/// Decoding a length five geohash:
///
/// ```rust
/// let geohash_str = "9q60y";
///
/// let decoded = geohash::decode(geohash_str).expect("Invalid hash string");
///
/// assert_eq!(
///     decoded,
///     (
///         geohash::Coordinate {
///             x: -120.65185546875,
///             y: 35.31005859375,
///         },
///         0.02197265625,
///         0.02197265625,
///     ),
/// );
/// ```
///
/// Decoding a length ten geohash:
///
/// ```rust
/// let geohash_str = "9q60y60rhs";
///
/// let decoded = geohash::decode(geohash_str).expect("Invalid hash string");
///
/// assert_eq!(
///     decoded,
///     (
///         geohash::Coordinate {
///             x: -120.66229999065399,
///             y: 35.300298035144806,
///         },
///         0.000005364418029785156,
///         0.000002682209014892578,
///     ),
/// );
/// ```
pub fn decode(hash_str: &str) -> Result<(Coordinate<f64>, f64, f64), GeohashError> {
    let rect = decode_bbox(hash_str)?;
    let c0 = rect.min();
    let c1 = rect.max();
    Ok((
        Coordinate {
            x: (c0.x + c1.x) / 2f64,
            y: (c0.y + c1.y) / 2f64,
        },
        (c1.x - c0.x) / 2f64,
        (c1.y - c0.y) / 2f64,
    ))
}

/// Find neighboring geohashes for the given geohash and direction.
pub fn neighbor(hash_str: &str, direction: Direction) -> Result<String, GeohashError> {
    let (coord, lon_err, lat_err) = decode(hash_str)?;
    let (dlat, dlng) = direction.to_tuple();
    let neighbor_coord = Coordinate {
        x: coord.x + 2f64 * lon_err.abs() * dlng,
        y: coord.y + 2f64 * lat_err.abs() * dlat,
    };
    encode(neighbor_coord, hash_str.len())
}

/// Find all neighboring geohashes for the given geohash.
///
/// ### Examples
///
/// ```
/// let geohash_str = "9q60y60rhs";
///
/// let neighbors = geohash::neighbors(geohash_str).expect("Invalid hash string");
///
/// assert_eq!(
///     neighbors,
///     geohash::Neighbors {
///         n: "9q60y60rht".to_owned(),
///         ne: "9q60y60rhv".to_owned(),
///         e: "9q60y60rhu".to_owned(),
///         se: "9q60y60rhg".to_owned(),
///         s: "9q60y60rhe".to_owned(),
///         sw: "9q60y60rh7".to_owned(),
///         w: "9q60y60rhk".to_owned(),
///         nw: "9q60y60rhm".to_owned(),
///     }
/// );
/// ```
pub fn neighbors(hash_str: &str) -> Result<Neighbors, GeohashError> {
    Ok(Neighbors {
        sw: neighbor(hash_str, Direction::SW)?,
        s: neighbor(hash_str, Direction::S)?,
        se: neighbor(hash_str, Direction::SE)?,
        w: neighbor(hash_str, Direction::W)?,
        e: neighbor(hash_str, Direction::E)?,
        nw: neighbor(hash_str, Direction::NW)?,
        n: neighbor(hash_str, Direction::N)?,
        ne: neighbor(hash_str, Direction::NE)?,
    })
}
