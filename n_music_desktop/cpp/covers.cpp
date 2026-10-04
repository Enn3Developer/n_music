#include "covers.h"

#include <QtCore/QUrl>
#include <QtGui/QImageReader>
#include <QtGui/QPainter>
#include <QtQuick/QQuickImageProvider>

namespace {

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
    const qsizetype slash = id.indexOf(u'/');
    const qreal radius = id.left(slash).toDouble();
    const QString path = QUrl::fromPercentEncoding(id.mid(slash + 1).toUtf8());

    QImageReader reader(path);
    reader.setAutoTransform(true);
    const QImage source = reader.read();
    if (source.isNull()) {
      return {};
    }

    QSize target = requestedSize;
    if (target.width() <= 0 && target.height() <= 0) {
      target = source.size();
    } else if (target.width() <= 0) {
      target.setWidth(target.height());
    } else if (target.height() <= 0) {
      target.setHeight(target.width());
    }

    // Fill the target and crop what overflows, like a square cover frame.
    const QImage scaled = source.scaled(
      target, Qt::KeepAspectRatioByExpanding, Qt::SmoothTransformation);
    const QImage cropped =
      scaled.copy((scaled.width() - target.width()) / 2,
                  (scaled.height() - target.height()) / 2,
                  target.width(),
                  target.height());

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
