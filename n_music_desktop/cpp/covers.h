#pragma once

#include <QtQml/QQmlEngine>

/// Serves `image://cover/<radius>/<path>`: the cover thumbnail at `path` cropped to the
/// requested size, with corners rounded by `radius`, a fraction of the width.
void installCoverProvider(QQmlEngine& engine);
