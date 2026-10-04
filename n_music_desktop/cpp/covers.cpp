#include "covers.h"

#include <QtCore/QUrl>
#include <QtGui/QImageReader>
#include <QtGui/QPainter>
#include <QtQuick/QQuickImageProvider>

namespace {

/// The image at `path` filling `target`, cropped to it like a square cover frame.
QImage
fill(const QString& path, const QSize& target)
{
  QImageReader reader(path);
  reader.setAutoTransform(true);
  const QImage source = reader.read();
  if (source.isNull()) {
    return {};
  }
  const QImage scaled = source.scaled(
    target, Qt::KeepAspectRatioByExpanding, Qt::SmoothTransformation);
  return scaled.copy((scaled.width() - target.width()) / 2,
                     (scaled.height() - target.height()) / 2,
                     target.width(),
                     target.height());
}

/// The images at `paths` in a two by two grid filling `target`.
QImage
mosaic(const QStringList& paths, const QSize& target)
{
  QImage grid(target, QImage::Format_ARGB32_Premultiplied);
  grid.fill(Qt::transparent);
  QPainter painter(&grid);
  const int left = target.width() / 2;
  const int top = target.height() / 2;
  for (int i = 0; i < 4; ++i) {
    const QRect tile(i % 2 == 0 ? 0 : left,
                     i < 2 ? 0 : top,
                     i % 2 == 0 ? left : target.width() - left,
                     i < 2 ? top : target.height() - top);
    painter.drawImage(tile.topLeft(), fill(paths.at(i), tile.size()));
  }
  painter.end();
  return grid;
}

class CoverProvider final : public QQuickImageProvider
{
public:
  CoverProvider()
    : QQuickImageProvider(QQuickImageProvider::Image,
                          QQmlImageProviderBase::ForceAsynchronousImageLoading)
  {
  }

  QImage requestImage(const QString& id,
                      QSize* size,
                      const QSize& requestedSize) override
  {
    // `<radius>/<path>`, or four comma-separated paths for a mosaic; each path is
    // percent-encoded, so a comma in one does not split it. (URLs keep a comma as it is,
    // unlike a `|`.)
    const qsizetype slash = id.indexOf(u'/');
    const qreal radius = id.left(slash).toDouble();
    QStringList paths = id.mid(slash + 1).split(u',');
    for (QString& path : paths) {
      path = QUrl::fromPercentEncoding(path.toUtf8());
    }

    QSize target = requestedSize;
    if (target.width() <= 0 && target.height() <= 0) {
      target = QImageReader(paths.first()).size();
    } else if (target.width() <= 0) {
      target.setWidth(target.height());
    } else if (target.height() <= 0) {
      target.setHeight(target.width());
    }
    if (target.isEmpty()) {
      return {};
    }

    const QImage cropped =
      paths.size() == 4 ? mosaic(paths, target) : fill(paths.first(), target);
    if (cropped.isNull()) {
      return {};
    }

    // A texture fill, unlike a clip path, antialiases the rounded edge.
    QImage rounded(target, QImage::Format_ARGB32_Premultiplied);
    rounded.fill(Qt::transparent);
    QPainter painter(&rounded);
    painter.setRenderHint(QPainter::Antialiasing);
    painter.setPen(Qt::NoPen);
    painter.setBrush(QBrush(cropped));
    const qreal corner = radius * target.width();
    painter.drawRoundedRect(QRectF(QPointF(0, 0), QSizeF(target)), corner, corner);
    painter.end();

    if (size != nullptr) {
      *size = rounded.size();
    }
    return rounded;
  }
};

}

void
installCoverProvider(QQmlEngine& engine)
{
  engine.addImageProvider(QStringLiteral("cover"), new CoverProvider);
}
